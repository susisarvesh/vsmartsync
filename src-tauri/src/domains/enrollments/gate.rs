//! Process-local lock so one device runs one physical enrollment at a time.
//!
//! The lock is not stored in PostgreSQL. A restart releases it. Cancellation
//! does not release the lock until the in-flight `enrolluser` call returns.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::EnrollmentError;

#[derive(Clone)]
pub struct DeviceEnrollmentGate {
    active: Arc<Mutex<HashMap<Uuid, Uuid>>>,
}

impl DeviceEnrollmentGate {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, Uuid>> {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn try_acquire(&self, device_id: Uuid, session_id: Uuid) -> Result<(), EnrollmentError> {
        let mut active = self.lock();
        if active.contains_key(&device_id) {
            return Err(EnrollmentError::DeviceBusy);
        }
        active.insert(device_id, session_id);
        Ok(())
    }

    pub fn release(&self, device_id: Uuid, session_id: Uuid) {
        let mut active = self.lock();
        if active.get(&device_id) == Some(&session_id) {
            active.remove(&device_id);
        }
    }
}

impl Default for DeviceEnrollmentGate {
    fn default() -> Self {
        Self::new()
    }
}

/// Releases the device when the enrollment task ends, including on cancel.
pub struct DeviceEnrollmentGuard {
    gate: DeviceEnrollmentGate,
    device_id: Uuid,
    session_id: Uuid,
}

impl DeviceEnrollmentGuard {
    pub fn acquire(
        gate: &DeviceEnrollmentGate,
        device_id: Uuid,
        session_id: Uuid,
    ) -> Result<Self, EnrollmentError> {
        gate.try_acquire(device_id, session_id)?;
        Ok(Self {
            gate: gate.clone(),
            device_id,
            session_id,
        })
    }
}

impl Drop for DeviceEnrollmentGuard {
    fn drop(&mut self) {
        self.gate.release(self.device_id, self.session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceEnrollmentGate;
    use crate::domains::enrollments::EnrollmentError;
    use uuid::Uuid;

    #[test]
    fn second_session_on_the_same_device_is_busy() {
        let gate = DeviceEnrollmentGate::new();
        let device = Uuid::new_v4();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        gate.try_acquire(device, first).unwrap();
        assert_eq!(
            gate.try_acquire(device, second).unwrap_err(),
            EnrollmentError::DeviceBusy
        );
    }

    #[test]
    fn different_devices_can_enroll_together() {
        let gate = DeviceEnrollmentGate::new();
        gate.try_acquire(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        gate.try_acquire(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    }

    #[test]
    fn release_allows_the_next_session() {
        let gate = DeviceEnrollmentGate::new();
        let device = Uuid::new_v4();
        let first = Uuid::new_v4();
        gate.try_acquire(device, first).unwrap();
        gate.release(device, first);
        gate.try_acquire(device, Uuid::new_v4()).unwrap();
    }

    #[test]
    fn guard_drop_releases_only_its_session() {
        let gate = DeviceEnrollmentGate::new();
        let device = Uuid::new_v4();
        let session = Uuid::new_v4();
        let guard = super::DeviceEnrollmentGuard::acquire(&gate, device, session).unwrap();
        drop(guard);
        gate.try_acquire(device, Uuid::new_v4()).unwrap();
    }
}
