## Release 1.1.0
New function supporting search for Quote ticker has been added, which required an additional URL path to access the Yahoo API. The previously single file project has been split up into separate files for improved maintainability.

**Note**: Yahoo-Error type has changed. `FetchFailed` has now a string as argument instead of the status code passed over by `reqwest` to decouple the interface from `reqwest`. The former error code `InvalidStatusCode` has been renamed to `InvalidJson`, which a more proper name since this error is returned if the response could not be read as JSON. 

# Release 1.0.0
The library is working stable with and without blocking feature enabled.