# `lambda-ddb-gateway`

Provide a simple `GET`/`SET` interface to DynamoDB.

- `GET` is unauthenticated
- `SET` depends on having the correct token

## Notes

- This code doesn't automatically provision any tables; that must happen
externally.

- We substantially simplify the schema. While DynamoDB is capable of
having multiple keys of varying types and complex values, this project assumies
that keys and values are always strings.

- No length limit is applied to keys.
