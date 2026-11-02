-- What a paired device is (add-remote-desktop-client D1): `phone` or `desktop`. Devices
-- paired before this were phones.
ALTER TABLE remote_devices ADD COLUMN kind TEXT NOT NULL DEFAULT 'phone';
