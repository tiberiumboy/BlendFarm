Expected behaviour - Whenever client node completes a render image, before broadcasting out to the network for status update, a new record is appended to database containing information related to the job task. This information should be persistent across app lifespan.

Actual Behaviour - No data is saved to the database.