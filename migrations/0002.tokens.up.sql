CREATE TABLE tokens (
	id		uuid PRIMARY KEY,
	user_id	uuid REFERENCES users(id),
	created	timestamp NOT NULL,
	device_id	text NOT NULL
);
