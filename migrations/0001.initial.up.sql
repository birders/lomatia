CREATE TABLE users (
	id		uuid,
	localpart	varchar(255),
	passhash	varchar(60),
	CONSTRAINT users_id_pkey PRIMARY KEY(id)
);
