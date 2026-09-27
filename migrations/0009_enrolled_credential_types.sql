-- Credential types recorded after a hardware enrollment.
-- These rows mark that the device holds the credential. They are not template bytes.
ALTER TABLE credentials DROP CONSTRAINT credentials_type_allowed;

ALTER TABLE credentials
    ADD CONSTRAINT credentials_type_allowed CHECK (
        type IN (
            'card',
            'pin',
            'read_only_card',
            'smart_card',
            'finger',
            'face',
            'duress_finger',
            'biometric_card'
        )
    );
