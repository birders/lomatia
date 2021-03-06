<p align=center><img src="img/icon.png" width="20%"></p>

<h1 align="center">Lomatia</h1>

Lomatia is a Matrix homeserver written entirely in Rust. It is currently WIP,
and aims to implement the latest revision of the Matrix protocol.

## Features

 - Core API
    - User data
       - [ ] List third party identifiers
       - [ ] Add contact information to account
       - [ ] Deactivate account
       - [ ] Change account password
       - [ ] Get information about owner of access token
       - [ ] Get profile information
       - [ ] Get avatar URL
       - [ ] Set avatar URL
       - [ ] Get display name
       - [ ] Set display name
       - [x] Register account
       - [ ] Set account data for user
       - [ ] Set account data for user, room-specific
       - [ ] List tags for room
       - [ ] Remove a tag from a room
       - [ ] Add a tag to a room
    - Server administration
       - [ ] Get information about user
       - [x] Get versions of specification supported by the server
    - Room creation
       - [ ] Create a new room
    - Device management
       - [ ] List registered devices for the current user
       - [ ] Delete a device
       - [ ] Get a single device
       - [ ] Update a device
    - Room directory
       - [ ] Remove a mapping of room alias to room ID
       - [ ] Get the room ID corresponding to this room alias
       - [ ] Create a new mapping from room alias to room ID
    - Room participation
       - [ ] Get events and state around the specified event
       - [ ] Get the list of currently joined users and their profile data
       - [ ] Get the list of all users and their profile data
       - [ ] Get the list of events for this room
       - [ ] Send a receipt for the given event ID
       - [ ] Strip all non-integrity-critical information out of an event
       - [ ] Send a message event to the given room
       - [ ] Get all state events in the current state of a room
       - [ ] Get the state identified by the type with the empty state key
       - [ ] Send a state event to the given room
       - [ ] Get the state identified by the type and key
       - [ ] Send a state event to the given room, with state key
       - [ ] Get the state identified by the type and key
       - [ ] Synchronise the client's state and receive new messages
       - [ ] Upload a new filter
       - [ ] Download a filter
    - Room membership
       - [ ] Start the requesting user participating in a particular room
       - [ ] List the user's current rooms
       - [ ] Ban a user in the room
       - [ ] Stop the requesting user remembering about a particular room
       - [ ] Invite a user to participate in a particular room, via third party
	   endpoint
       - [ ] Invite a user to participate in a particular room, via user ID
	   endpoint
       - [ ] Start the requesting user participating in a particular room
       - [ ] Kick a user from the room
       - [ ] Stop the requesting user participating in a particular room
       - [ ] Unban a user from the room
    - End-to-end encryption
       - [ ] Query users with recent device key updates
       - [ ] Claim one-time encryption keys
       - [ ] Download device identity keys
       - [ ] Upload end-to-end encryption keys
    - Session management
       - [ ] Login (Authenticate the user)
       - [ ] Logout (Invalidate an access token)
    - Push notifications
       - [ ] Get a list of events the user has been notified about
       - [ ] Get the current pushers for the authenticated user
       - [ ] Modify a pusher for this user on the homeserver
       - [ ] Retrieve all push requests
       - [ ] Delete a push request
       - [ ] Retrieve a push rule
       - [ ] Add or change a push rule
       - [ ] Set the actions for a push rule
       - [ ] Enable or disable a push rule
    - Presence
       - [ ] Get presence events for this presence list
       - [ ] Add or remove users from this presence list
       - [ ] Get this user's presence state
       - [ ] Update this user's presence state
    - Room discovery
       - [ ] List the public rooms on a server
       - [ ] List the public rooms on the server with optional filter
    - Search
       - [ ] Perform a server-side search
    - Send-to-Device messaging
       - [ ] Send an event to a given set of devices
    - VOIP
       - [ ] Obtain TURN server credentials
    - Media
       - [ ] Download the content from the content repository
       - [ ] Download content from the content repository as a given filename
       - [ ] Get information about a URL for a client
       - [ ] Download a thumbnail of the content from the content repository
       - [ ] Upload some content to the content repository
