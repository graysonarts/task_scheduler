# Requirements

Postgres, Rust Stable 1.77.1

# Running

## prepare the database

1. Edit `.env` to point to your postgres database
2. `sqlx create database`
3. `sqlx migrate run`

## start the worker (as many as you need)

`cargo run --bin worker`

## start the api (as many as you need, but you'll need a load balancer in front)

`cargo run --bin scheduler`

# Developing

You need to have a local running database setup to compile. I just used docker to run postgres locally. This is because sqlx does compile time validation of queries. There is an offline mode, but I ran out of time to implement that.

## Running the database tests

I have some tests that run against the live database. **They will delete everything in the database**.

They are behind a feature flag, and are required to be run serially. Luckily, there is the `serial-test` crate that can achieve that goal.

Run the tests like this: `cargo test --package common --features real_database_tests`

# API

_Note: all times are in `ISO-8601` format with `+0000` replaced with `Z` for UTC._

- `GET /tasks` get a list of the tasks
  - Add `?filter=status:<status>` to filter by status
    - Valid values are `Pending`, `InProgress`, and `Completed` (case-sensitive)
  - Add `?filter=kind:<kind>` to filter by kind
    - Valid values are `Foo`, `Bar`, and `Baz` (case-sensitive)
  - Filters _cannot_ be combined in this first version. Maybe in future versions, but there was not enough time to implement that.
- `PUT /tasks` create a task
  - Example Payload: `{ "kind": "Foo", "execute_at": "2024-03-30T23:17:10.790146Z" }`
  - Example Response Body: `{ "id": "f6490a75-c7ec-437d-8e2b-f8b82b9f3617" }`
  - The body should be json with a `Content-Type: application/json` header
  - Tasks are automatically created in the pending state and scheduled to execute at the appropriate time
- `GET /tasks/<id>` get a specific task
  - Example Response Body: `{"id": "f6490a75-c7ec-437d-8e2b-f8b82b9f3617", "kind": "Foo", "process_at": "2024-03-29T23:17:10.790146Z","status": "Pending" }`
  - Note that process*at is the time in which it's scheduled to run \_at the earliest*
- `DELETE /tasks/<id>` deletes the task from the list
  - Will return `204 No Content` on successful delete

# Future Improvements

- Tests, there are some to prove out the database, and run against a live database
- Ideally, using a sql database as a queue is not advisable. Should we be able to use an actual message queue for notifying the worker that new work exists, that would be the prefered solution. Something like rabbitMQ or if we want to tie ourselves to AWS, SQS, would be good. To implement that, I would still store the work in postgres, and then kick off a message with the work id. This has the trade off of having the worker look at work and requeue if the processing time has not been met yet.
- Error handling is a bit course because I didn't want to spend time making it elegant. I don't give the end user or the server logs much details about what failed. In a production system, I would want to at least have extensive logging of errors.
- Logging using the trace library. I skipped this because I didn't feel it was necessary for this test, but in a production environment, I'd used structured logging in json format for better usability with log collection tools
