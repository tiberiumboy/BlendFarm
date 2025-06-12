-- used to create records of previous advertisement in case of a unexpected shutdown. avoid memory usage as much as you can.
CREATE TABLE IF NOT EXISTS advertise(
    id TEXT NOT NULL UNIQUE,    -- primary key
    -- See if we need anything special, but for now, just name and path both of which is protected keywords.
    ad_name TEXT NOT NULL, -- name we broadcast
    file_path TEXT NOT NULL -- path to file to respond
)