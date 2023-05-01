1. We'll have 2 additional tables, user and auth
2. User table will have a password and a username field minimum. Password should be hashed.
3. Auth table will have a user_id foreign key and a random token
4. Todo will have user_id foreign key instead of username field.
5. 2 new apis - Login, register.
6. These 2 apis will be unauthenticated.
7. Other apis will be authenticated (via a middleware). In the middleware a request header (ex: "X-Session-Token") will be inspected and corresponding user will be found from the token table.
