-- Add migration script here
CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    username VARCHAR(32) UNIQUE,
    `password` VARCHAR(32)
);

CREATE TABLE share (
    id INTEGER PRIMARY KEY,
    `path` VARCHAR UNIQUE,
    `url` VARCHAR,
    `password` VARCHAR
);

CREATE INDEX share_index ON share (`path`, `url`);