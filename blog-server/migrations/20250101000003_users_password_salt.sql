-- В БД явно хранится соль (строка SaltString для Argon2); password_hash по-прежнему полная PHC-запись.
-- Пустое значение у старых строк после миграции: вход опирается только на password_hash.
ALTER TABLE users
    ADD COLUMN password_salt TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN users.password_salt IS 'случайная соль пользователя для Argon2 (дубликат параметра для наглядности в SQL)';
