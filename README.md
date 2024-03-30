## prepare the database

1. Edit `.env` to point to your postgres database
2. `sqlx create database`
3. `sqlx migrate run`

## Future Improvements

- Ideally, using a sql database as a queue is not advisable. Should we be able to use an actual message queue for notifying the worker that new work exists, that would be the prefered solution. Something like rabbitMQ or if we want to tie ourselves to AWS, SQS, would be good. To implement that, I would still store the work in postgres, and then kick off a message with the work id. This has the trade off of having the worker look at work and requeue if the processing time has not been met yet.
- Error handling is a bit course because I didn't want to spend time making it elegant. I don't give the end user or the server logs much details about what failed. In a production system, I would want to at least have extensive logging of errors.
- Logging using the trace library. I skipped this because I didn't feel it was necessary for this test, but in a production environment, I'd used structured logging in json format for better usability with log collection tools
