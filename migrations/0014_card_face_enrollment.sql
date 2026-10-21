-- Card + Face and Face + Card stay one enrollment session.
-- capture_factor tells the dialog which prompt is active. It is not a credential.

ALTER TABLE enrollment_sessions
    DROP CONSTRAINT enrollment_sessions_enroll_type_allowed;

ALTER TABLE enrollment_sessions
    ADD CONSTRAINT enrollment_sessions_enroll_type_allowed CHECK (
        enroll_type IN (
            'read_only_card',
            'smart_card',
            'biometric',
            'biometric_then_card',
            'face',
            'duress_finger',
            'card_then_face',
            'face_then_card'
        )
    );

ALTER TABLE enrollment_sessions
    ADD COLUMN capture_factor TEXT NULL;

ALTER TABLE enrollment_sessions
    ADD CONSTRAINT enrollment_sessions_capture_factor_allowed CHECK (
        capture_factor IS NULL OR capture_factor IN ('card', 'face')
    );
