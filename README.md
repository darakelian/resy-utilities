# About this repo
This repo contains various utilities for interacting with the Resy API programmatically. The project is split up
into a library containing the code for making the API calls then CLI tools that utilize the library. This project
currently is NOT meant to map the API completely; only endpoints/data types that are needed for the CLI tools will
be implemented.

# Interacting with the Resy API
Most requests to the Resy API require authentication tokens. This tokens are not explicitly presented to users
anywhere on the website and must be retrieved manually through network inspection tools such as a browser's built-in
network traffic analyzer. You need to extract two tokens from the requests if you wish to communicate with the
API using these tools:

|CLI Arg|Header Name|Value Needed|
|-------|-----------|------------|
|auth-token|Authentication|Value in the quotes after api_key|
|api-key|X-Resy-Auth-Token|The entire value|

It does not seem like the `X-Resy-Universal-Auth` header is important.

It is recommended to store these values as environment variables rather than passing them as CLI args to avoid
leaking them in plaintext. Be cautious, if these tokens are leaked it would allow someone to make requests on
your behalf.

# Copyright
The repo is licensed under the Apache 2.0 license (license details can be found in the LICENSE file.) No implied ownership of rights, trademarks, or licenses of Resy are implied to be transfered by this repo or usage of the
libraries and/or tools. All rights are reserved by Resy.