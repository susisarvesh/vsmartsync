# Matrix Vsmart current flow (Notion reference)

**Status:** REFERENCE SNAPSHOT. This is the current product flow. Existing docs under `docs/` still describe the previous flow and have not been reconciled with this page. Update those files one by one later. Do not treat this snapshot as implemented code.

**Source:** [Matrix Vsmart Documentation](https://leeward-scooter-7d7.notion.site/Matrix-Vsmart-Documentation-3cfa1e066080808f95f1f96091648a9a)

**Captured:** 2026-09-27

---

# Matrix COSEC Integration Platform — MVP Documentation

## 1. Overview

Our software acts as a **client application that communicates directly with Matrix COSEC hardware through the Matrix COSEC Device APIs**.

The purpose of this platform is to provide the customer with the core functionality required to interact with and manage Matrix COSEC devices without requiring the complete Matrix software platform.

The customer already has an existing enterprise system such as **SAP or another HR/attendance system** that can handle business-level functionality such as attendance calculation, reporting, payroll, and related operations. Therefore, the customer primarily requires a software layer that can:

- Manage Matrix COSEC devices.
- Manage users.
- Manage credentials.
- Perform enrollment through the hardware.
- Synchronize users and credentials with devices.
- Maintain device and user relationships.
- Handle synchronization when devices are temporarily unavailable.
The Matrix COSEC Device API documentation confirms that third-party applications can directly access and monitor supported COSEC devices through HTTP APIs without installing the COSEC server/Monitor.     COSEC DEVICES API GUIDE(3)


---

# 2. MVP Objective

The objective of the MVP is to build a lightweight platform that provides the **core device and user management functionality required to operate Matrix COSEC hardware**.

The MVP is not intended to replace the customer's complete HR, payroll, attendance, or ERP system.

Instead, the platform acts as a **hardware integration layer** between Matrix COSEC devices and the customer's existing business systems.

### High-Level Architecture

```text
                    ┌─────────────────────────┐
                    │   Customer Enterprise   │
                    │                         │
                    │ SAP / HR / Attendance   │
                    │ / Payroll / Other Apps  │
                    └────────────┬────────────┘
                                 │
                                 │ Integration
                                 ▼
                    ┌─────────────────────────┐
                    │   Our COSEC Platform    │
                    │                         │
                    │ User Management         │
                    │ Device Management       │
                    │ Credential Management   │
                    │ Enrollment              │
                    │ Synchronization          │
                    └────────────┬────────────┘
                                 │
                         Matrix COSEC API
                                 │
                ┌────────────────┼────────────────┐
                ▼                ▼                ▼
          ┌──────────┐     ┌──────────┐     ┌──────────┐
          │ Device 1 │     │ Device 2 │     │ Device 3 │
          └──────────┘     └──────────┘     └──────────┘
```


---

# 3. MVP Modules

The MVP consists of the following core modules:

1. **User Module**
1. **Device Module**
1. **Credential Module**
1. **Enrollment Module**
1. **Synchronization Module**
1. **Scheduler / Pending Job Module**
The User and Device modules form the primary management layer, while Credential, Enrollment, and Synchronization provide the functionality required to actually provision users onto COSEC hardware.


---

# 4. Functional Requirements

## 4.1 User Management

The system must allow an administrator to:

- Create a user.
- View users.
- Update users.
- Activate a user.
- Deactivate a user.
- Assign a user to one or more devices.
- View the credentials associated with a user.
- View the devices to which a user is assigned.
A user should have a unique system-generated ID.


---

## 4.2 Device Management

The system must allow an administrator to:

- Create/register a Matrix COSEC device.
- Store the device's network information.
- Store the device MAC address.
- Authenticate with the device.
- Check device availability.
- Retrieve device configuration.
- Determine available device capabilities.
- Maintain device status.
- Support multiple devices.
The Matrix API guide confirms that COSEC Device APIs are accessed directly using the device IP/port and that the APIs are dependent on the device type.     COSEC DEVICES API GUIDE(3)


---

## 4.3 User–Device Relationship

A user can be assigned to **multiple devices**.

For example:

```text
User: John

Assigned Devices:
    ├── Main Gate
    ├── Production Floor
    └── Office Entrance
```

Therefore, the relationship should not be stored as a single `device_id` inside the user.

Conceptually:

```text
USER
  │
  ├────────── DEVICE 1
  │
  ├────────── DEVICE 2
  │
  └────────── DEVICE 3
```

This allows the same employee to have credentials provisioned across multiple COSEC devices.


---

# 5. User Entity

The initial User entity should contain the following fields:

| Field | Type | Description |
| --- | --- | --- |
| `id` | UUID / internal ID | Automatically generated unique identifier |
| `username` | String | Name/username of the user |
| `status` | Enum | `ACTIVE` or `INACTIVE` |
| `created_at` | Timestamp | User creation time |
| `updated_at` | Timestamp | Last modification time |

### Default Status

When a user is created:

```text
status = ACTIVE
```

The administrator can later change the status to:

```text
ACTIVE
INACTIVE
```

### Example

```text
User
────────────────────────────
ID          : 8f3c...
Username    : John Doe
Status      : ACTIVE
Created At  : 2026-09-26
Updated At  : 2026-09-26
```


---

# 6. Device Entity

The Device entity should contain the following information:

| Field | Description |
| --- | --- |
| `id` | Automatically generated internal device ID |
| `device_name` | Friendly name given to the device |
| `host` | Device IP address / hostname |
| `port` | Device API port |
| `mac_address` | MAC address of the COSEC device |
| `device_model` | Model/type identified from device information/capability discovery |
| `status` | Current software/device status |
| `created_at` | Device creation timestamp |
| `updated_at` | Last modification timestamp |
| `last_seen_at` | Last successful communication with the device |

### Example

```text
Device
────────────────────────────────────
ID              : 4b2a...
Device Name     : Main Gate
Host            : 192.168.1.100
Port            : 80
MAC Address     : XX:XX:XX:XX:XX:XX
Device Model    : COSEC NGT
Status          : ACTIVE
Last Seen       : 2026-09-26 15:42:10
```


---

# 7. Device Authentication

The Matrix COSEC API uses **HTTP Basic Authentication**.

The Matrix API guide specifies:

```text
Username: admin
Password: password configured on the device
```

The documentation does **not** state that `1234` is the universal default password. Therefore, the product documentation should not hard-code `1234` as the default unless the specific device installation documentation confirms it.     COSEC DEVICES API GUIDE(3)

### Important Security Requirement

The device password should **not be requested as ordinary user-facing data unnecessarily**, and it should never be exposed to the React/UI layer.

The architecture should instead be:

```text
React UI
   │
   │ Device configuration
   ▼
Tauri Command
   │
   ▼
Rust Device Service
   │
   ▼
Encrypted Credential Storage
   │
   ▼
Matrix API Client
   │
   ▼
COSEC Device
```

The password should be securely stored and only decrypted when required for communication with the device.


---

# 8. Credential Module

Credentials represent the authentication methods belonging to a user.

A credential should be treated as a separate entity rather than simply storing credential information directly inside the User record.

### Credential Entity

| Field | Description |
| --- | --- |
| `id` | Automatically generated credential ID |
| `credential_type` | Type of credential |
| `user_id` | User associated with the credential |
| `device_id` | Device associated with the credential |
| `credential_model` | Credential/model information returned or required by the COSEC API |
| `created_at` | Creation timestamp |
| `updated_at` | Modification timestamp |


---

# 9. Supported Credential Types

The Matrix API documentation defines enrollment types including:

- Read-only card
- Smart card
- Biometric
- Biometric + card
- Face
- Duress finger
The exact availability depends on the device and its configured hardware.     COSEC DEVICES API GUIDE(3)

Therefore, the application should **not assume that every device supports every credential type**.

Instead, the application should discover the device's capabilities and display only the supported enrollment options.

For example:

```text
Device: COSEC NGT

Available Enrollment:
☑ Fingerprint
☑ Card
☐ Face
☐ Palm
```

Another device might expose:

```text
Device: COSEC ARGO FACE

Available Enrollment:
☑ Face
☑ Fingerprint
☑ Card
```

The API guide explicitly notes that the documentation covers multiple variants and that a particular product may not support every feature described.     COSEC DEVICES API GUIDE(3)


---

# 10. Credential–User Relationship

Every credential must belong to a user.

```text
USER
  │
  ├── Credential 1
  ├── Credential 2
  └── Credential 3
```

For example:

```text
John Doe
│
├── Fingerprint
├── RFID Card
└── Face
```

This allows the software to maintain a complete representation of the credentials belonging to a user.


---

# 11. Credential–Device Relationship

A credential also needs to be associated with the device on which it exists or is enrolled.

Therefore:

```text
User
 │
 └── Credential
       │
       └── Device
```

For example:

```text
John Doe
   │
   └── Fingerprint
          │
          ├── Main Gate
          ├── Production Gate
          └── Office Entrance
```

This becomes important when the same user is enrolled on multiple devices.


---

# 12. Enrollment Module

The Enrollment Module is responsible for initiating credential enrollment through the physical COSEC device.

The Matrix API provides an `enrolluser` API specifically for initiating enrollment on the device. The API allows the client application to specify the user and credential type, after which the device initiates the physical enrollment process.     COSEC DEVICES API GUIDE(3)

### Enrollment Flow

```text
Administrator
      │
      ▼
Select User
      │
      ▼
Select Device
      │
      ▼
Select Credential Type
      │
      ▼
Start Enrollment
      │
      ▼
Our Software
      │
      │ Matrix API
      ▼
COSEC Device
      │
      ▼
Physical Enrollment
      │
      ├── Finger
      ├── Face
      ├── Card
      └── Other supported credential
      │
      ▼
Enrollment Completed
      │
      ▼
Credential Associated
      │
      ├── User
      └── Device
```


---

# 13. Device Capability Discovery

When a device is added, the software should communicate with the device and retrieve its configuration.

The Matrix API provides APIs for:

- Basic device configuration
- Reader configuration
- Finger reader configuration
- Palm sensor configuration
- Enrollment configuration
- Other device configuration areas
The `device-basic-config` API can retrieve basic configuration such as application type, device name, additional security code, and maximum finger templates.     COSEC DEVICES API GUIDE(3)

The `enroll-options` API exposes enrollment-related configuration such as whether enrollment on the device is enabled and finger enrollment settings.     COSEC DEVICES API GUIDE(3)

Therefore, the application can build a **device capability profile** after connecting to a device.

Conceptually:

```text
Add Device
     │
     ▼
Authenticate
     │
     ▼
Basic Device Configuration
     │
     ├── Device information
     └── Finger capacity
     │
     ▼
Reader Configuration
     │
     ├── Reader type
     └── Reader capabilities
     │
     ▼
Enrollment Configuration
     │
     └── Enrollment capabilities
     │
     ▼
Capability Profile
```


---

# 14. Synchronization Module

The Synchronization Module ensures that the software database and physical COSEC devices remain consistent.

For example:

```text
Database
   │
   │ User created
   ▼
Sync Engine
   │
   ▼
Device
```

If the device is online:

```text
Database
   │
   ▼
Sync Job
   │
   ▼
COSEC Device
   │
   ▼
SUCCESS
```

If the device is offline:

```text
Database
   │
   ▼
Sync Job
   │
   ▼
Device Offline
   │
   ▼
PENDING
```

The synchronization operation must remain in the system and should not be lost.


---

# 15. Pending Synchronization

Each synchronization operation should have a state.

For example:

```text
PENDING
    ↓
PROCESSING
    ↓
SUCCESS
```

or:

```text
PENDING
    ↓
PROCESSING
    ↓
FAILED
    ↓
RETRY
    ↓
SUCCESS
```

This allows the system to reliably handle temporary device failures.


---

# 16. Scheduler / Retry Mechanism

If a device becomes inactive or unreachable, pending synchronization operations should be handled by a scheduler.

Example:

```text
                 ┌───────────────┐
                 │  Sync Request │
                 └───────┬───────┘
                         │
                         ▼
                 ┌───────────────┐
                 │ Device Online?│
                 └───────┬───────┘
                    YES  │  NO
                         │
             ┌───────────┘
             │
             ▼
        ┌──────────┐       ┌──────────────┐
        │   Sync   │       │ Create/Persist│
        │  Device  │       │ Pending Job   │
        └────┬─────┘       └───────┬──────┘
             │                     │
             ▼                     ▼
          SUCCESS             Retry Scheduler
                                   │
                                   ▼
                            Device Available?
                                   │
                                   ▼
                              Retry Sync
```


---

# 17. Complete MVP Flow

The complete MVP workflow can therefore be represented as:

```text
                         ADMINISTRATOR
                              │
                              ▼
                    ┌───────────────────┐
                    │   USER MODULE     │
                    │                   │
                    │ Create User       │
                    │ Update User       │
                    │ Activate/Deactivate│
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  DEVICE MODULE    │
                    │                   │
                    │ Add Device        │
                    │ IP + Port         │
                    │ MAC Address       │
                    │ Authentication    │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ CAPABILITY        │
                    │ DISCOVERY         │
                    │                   │
                    │ Device Config     │
                    │ Reader Config     │
                    │ Enrollment Config │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ USER ↔ DEVICE     │
                    │ ASSIGNMENT        │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ CREDENTIAL MODULE │
                    │                   │
                    │ Card              │
                    │ Fingerprint       │
                    │ Face              │
                    │ Other supported   │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ ENROLLMENT MODULE │
                    │                   │
                    │ Start enrollment  │
                    │ through COSEC     │
                    │ hardware          │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │ SYNC ENGINE       │
                    │                   │
                    │ DB → Devices      │
                    └─────────┬─────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
              DEVICE ONLINE       DEVICE OFFLINE
                    │                   │
                    ▼                   ▼
               Sync Now            Pending Job
                                        │
                                        ▼
                                  Retry Scheduler
                                        │
                                        ▼
                                  Sync When Online
```


---

# 18. Core Data Relationships

The fundamental domain relationships are:

```text
                    ┌─────────────┐
                    │    USER     │
                    └──────┬──────┘
                           │
                    assigned to
                           │
                           ▼
                    ┌─────────────┐
                    │   DEVICE    │
                    └──────┬──────┘
                           │
                       enrolled
                           │
                           ▼
                    ┌─────────────┐
                    │ CREDENTIAL  │
                    └──────┬──────┘
                           │
                      belongs to
                           │
                           ▼
                         USER
```

More accurately, because a user can have multiple devices and credentials:

```text
             ┌──────────────┐
             │     USER     │
             └──────┬───────┘
                    │
           ┌────────┴────────┐
           │                 │
           ▼                 ▼
     User-Device        Credentials
     Assignment             │
           │                │
           ▼                ▼
       ┌────────┐      ┌────────────┐
       │ DEVICE │      │ CREDENTIAL │
       └────────┘      └────────────┘
              \             /
               \           /
                \         /
                 └──Enrollment──┘
```

This structure will allow the system to scale beyond the initial MVP without having to redesign the fundamental domain model.


---

# 19. MVP Success Criteria

The MVP will be considered functionally complete when the following workflow works end-to-end:

```text
1. Create User
        ↓
2. Create/Register Device
        ↓
3. Authenticate with Device
        ↓
4. Discover Device Configuration/Capabilities
        ↓
5. Assign User to Device
        ↓
6. Select Credential Type
        ↓
7. Start Enrollment
        ↓
8. User enrolls on physical COSEC device
        ↓
9. Credential is associated with User + Device
        ↓
10. Synchronization state is maintained
        ↓
11. If Device is Offline → Pending Sync Job
        ↓
12. Device becomes Online
        ↓
13. Scheduler retries synchronization
        ↓
14. User and Device become synchronized
```

This gives the MVP a clear boundary: **the platform manages the relationship between users, credentials, and Matrix COSEC hardware, while the customer's existing SAP/HR/attendance system can remain responsible for business-level attendance and enterprise processing.**

# Matrix COSEC Integration — API Flow

Below is the **clean MVP API flow** for your application, separated into **Matrix APIs** and **your application/database operations**.


---

## 1. Add Device

**Flow**

```text
Admin
  ↓
Enter Device IP / Host / Port / Password
  ↓
Connect to COSEC Device
```

**API**

```text
GET /device.cgi/device-basic-config?action=get
```

**Example**

```text
GET http://192.168.1.100/device.cgi/device-basic-config?action=get
```

**Purpose**

- Verify the device is reachable
- Authenticate with Matrix device
- Retrieve basic device configuration
**Authentication**

```text
Username: admin
Password: Password configured on the device
```

COSEC DEVICES API GUIDE(3)

↓


---

## 2. Discover Device Configuration

**Flow**

```text
Connected Device
  ↓
Read Device Configuration
```

### API — Basic Configuration

```text
GET /device.cgi/device-basic-config?action=get
```

Used for:

- Device name
- Basic device configuration
- Application/device configuration information
COSEC DEVICES API GUIDE(3)

### API — Reader Configuration

```text
GET /device.cgi/reader-config?action=get
```

Used to determine:

- Reader configuration
- Configured reader types
↓


---

## 3. Discover Enrollment Capabilities

**Flow**

```text
Device Configuration
  ↓
Reader Configuration
  ↓
Enrollment Configuration
  ↓
Build Capability Profile
```

**API**

```text
GET /device.cgi/enroll-options?action=get
```

Used for:

- Enrollment configuration
- Enrollment method
- Finger configuration
- Finger count
- Template configuration
COSEC DEVICES API GUIDE(3)

> **Important:** These APIs can help your application determine the device's configured capabilities, but the guide does not document a dedicated API that reliably returns the exact hardware model.

↓


---

# 4. Create User

This is an **application/database operation**, not a Matrix API.

**Flow**

```text
Admin
  ↓
Create User
  ↓
PostgreSQL
```

Example:

```text
{
  "username": "John Doe",
  "status": "ACTIVE"
}
```

↓


---

# 5. Assign User to Device

Also an **application/database operation**.

Because one user can exist on multiple devices:

```text
User
 ├── Device A
 ├── Device B
 └── Device C
```

Use a relationship table such as:

```text
user_devices
```

Example:

```text
user_id   device_id
--------  ---------
U001      D001
U001      D002
```

↓


---

# 6. Create User on COSEC Device

Now your application synchronizes the local user to the physical device.

**API**

```text
users
```

Used for:

- Create user
- Update user
- Delete user
- Configure user
The exact parameters depend on the Matrix API operation/device support.     COSEC DEVICES API GUIDE(3)

**Flow**

```text
Application DB
      ↓
Sync Engine
      ↓
Matrix users API
      ↓
COSEC Device
```

↓


---

# 7. Start Credential Enrollment

Once the user exists on the device, start physical enrollment.

**API**

```text
/device.cgi/enrolluser
```

### Fingerprint / Biometric

```text
GET /device.cgi/enrolluser?action=enroll&type=2&user-id=101
```

### Face

```text
GET /device.cgi/enrolluser?action=enroll&type=7&user-id=101
```

### Card

```text
GET /device.cgi/enrolluser?action=enroll&type=0&user-id=101
```

The API starts the enrollment process; the user then physically presents the credential to the device.

COSEC DEVICES API GUIDE(3)

↓


---

# 8. Physical Enrollment

```text
Application
     ↓
enrolluser API
     ↓
COSEC Device
     ↓
User presents:
 ├── Finger
 ├── Face
 └── Card
     ↓
Device captures credential
```

No separate API is required to physically capture the credential.

↓


---

# 9. Retrieve Credential

After enrollment, retrieve the user's credential information.

**API**

```text
/device.cgi/credential
```

Example:

```text
GET /device.cgi/credential?action=get&user-id=101
```

Used for:

- Retrieve credentials
- Configure credentials
- Delete credentials
COSEC DEVICES API GUIDE(3)

↓


---

# 10. Save Credential Relationship

This is your **local database operation**.

For example:

```text
Credential
-----------
id
user_id
device_id
credential_type
credential_index
status
created_at
updated_at
```

Relationship:

```text
User
  ↓
Credential
  ↓
Device
```

Example:

```text
John
 └── Fingerprint
      └── COSEC Device 01
```

↓


---

# 11. Synchronization

There is **no single Matrix "sync" API**.

Your Sync Engine orchestrates the Matrix APIs.

```text
Local DB
   ↓
Sync Engine
   ↓
Check Device
   ↓
users API
   ↓
credential API
   ↓
enrolluser API
   ↓
COSEC Device
```

### Example

```text
User created
     ↓
User assigned to Device 01
     ↓
Sync Job created
     ↓
users API
     ↓
User created on device
     ↓
Credential enrollment
     ↓
credential API
     ↓
SYNCED
```

↓


---

# 12. Device Availability

There is no dedicated health-check API documented in the guide.

Use an authenticated API such as:

```text
GET /device.cgi/device-basic-config?action=get
```

If the request succeeds:

```text
ONLINE
```

If connection fails:

```text
OFFLINE
```

↓


---

# 13. Pending Synchronization

If the device is offline:

```text
User Change
    ↓
Device Offline
    ↓
Create Sync Job
    ↓
PENDING
```

Example:

```text
sync_jobs
--------------------------------
id
user_id
device_id
operation
status
retry_count
last_error
created_at
updated_at
```

↓


---

# 14. Scheduler Retry

Your scheduler periodically processes:

```text
PENDING
   ↓
Check Device
   ↓
Online?
 ┌───────┴───────┐
 NO              YES
 ↓                ↓
WAIT          Execute API
                  ↓
               SUCCESS
```

If it fails:

```text
FAILED
  ↓
RETRY
  ↓
PENDING
```

↓


---

# 15. Update User

When a user changes in your application:

```text
User Updated
     ↓
Create Sync Job
     ↓
Device Online?
     ↓
users API
     ↓
COSEC Device Updated
     ↓
SYNCED
```

↓


---

# 16. Delete User / Credential

### Delete Credential

```text
/device.cgi/credential?action=delete&user-id=101
```

### Delete User

```text
/device.cgi/users?action=delete&user-id=101
```

The exact supported operations/parameters should be validated against the target device/API version.     COSEC DEVICES API GUIDE(3)

↓


---

# 17. Events — Later / Optional

If your application eventually needs attendance/event information:

### HTTP Events

```text
GET /device.cgi/events?action=getevent
```

### TCP Events

```text
GET /device.cgi/tcp-events?action=getevent
```

These are relevant if you later want your application to consume device events and forward them to SAP/another external system.     COSEC DEVICES API GUIDE(3)


---

# Complete MVP Flow

```text
                         ┌──────────────────┐
                         │      ADMIN       │
                         └────────┬─────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │   ADD DEVICE     │
                         └────────┬─────────┘
                                  │
                                  ▼
                   device-basic-config?action=get
                                  │
                                  ▼
                         ┌──────────────────┐
                         │ DEVICE CONNECTED │
                         └────────┬─────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
             reader-config              enroll-options
                    │                           │
                    └─────────────┬─────────────┘
                                  ▼
                       DEVICE CAPABILITIES
                                  │
                                  ▼
                         ┌──────────────────┐
                         │   CREATE USER    │
                         │    Local DB      │
                         └────────┬─────────┘
                                  │
                                  ▼
                         ASSIGN TO DEVICE
                                  │
                                  ▼
                            users API
                                  │
                                  ▼
                       USER ON COSEC DEVICE
                                  │
                                  ▼
                       START ENROLLMENT
                                  │
                                  ▼
                           enrolluser API
                                  │
                                  ▼
                     ┌──────────────────────┐
                     │ PHYSICAL ENROLLMENT  │
                     │ Finger / Face / Card │
                     └──────────┬───────────┘
                                │
                                ▼
                          credential API
                                │
                                ▼
                       SAVE CREDENTIAL
                            Local DB
                                │
                                ▼
                          SYNCHRONIZED
                                │
                    ┌───────────┴───────────┐
                    │                       │
                  ONLINE                  OFFLINE
                    │                       │
                    ▼                       ▼
              Execute Sync            Pending Job
                                            │
                                            ▼
                                         Scheduler
                                            │
                                            ▼
                                      Retry Sync
```

## MVP Matrix APIs

| API | Purpose | Required |
| --- | --- | --- |
| `device-basic-config` | Connect, authenticate, read basic configuration | **Yes** |
| `reader-config` | Determine reader configuration | **Yes** |
| `enroll-options` | Determine enrollment configuration | **Yes** |
| `users` | Create/update/delete users on device | **Yes** |
| `credential` | Get/set/delete credentials | **Yes** |
| `enrolluser` | Start physical enrollment | **Yes** |
| `events` | Retrieve device events | Later |
| `tcp-events` | Receive device events | Later |
| `command` | Device commands | Later |

### Your application's own components

```text
User DB
Device DB
User-Device Assignment
Credential DB
Enrollment
Sync Engine
Sync Jobs
Scheduler
Device Monitor
```

So the **core MVP is essentially 6 Matrix APIs**:

```text
device-basic-config
        ↓
reader-config
        ↓
enroll-options
        ↓
users
        ↓
enrolluser
        ↓
credential
```

Everything around these APIs—**users, device records, relationships, credentials metadata, enrollment records, synchronization, pending jobs, and scheduler—is your application's architecture**, not a Matrix API.
