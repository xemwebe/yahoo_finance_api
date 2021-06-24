## Release 1.2.0
Added support for dividends and stock splits, see the new examples for splits and dividends and some code clean-up.

## Release 1.1.5
Upgrade to version 0.4.* of tokio-test

## Release 1.1.4
Mainly bug fixes and exports added for most structs. 
`search_result_opt` has been added, since sometimes not all fields are returned. These has been replaced by `Option<...>` type fields. The interface
of the `search_result` is left untouched, but returns now a default value (e.g.) empty string instead of an error.

## Release 1.1.0
New function supporting search for Quote ticker has been added, which required an additional URL path to access the Yahoo API. The previously single file project has been split up into separate files for improved maintainability. Especially, the blocking and async implementations are now
in separate files.

**Note**: Yahoo-Error type has changed. `FetchFailed` has now a string as argument instead of the status code passed over by `reqwest` to decouple the interface from `reqwest`. The former error code `InvalidStatusCode` has been renamed to `InvalidJson`, which a more proper name since this error is returned if the response could not be read as JSON. 

# Release 1.0.0
The library is working stable with and without blocking feature enabled.