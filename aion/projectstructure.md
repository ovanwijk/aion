# Project structure

The basis of the application consists of three components. 

1) The storage layer, a rocks DB implemententation that handles all local persistence code (package indexstorage)
2) The actor syster. A riker.rs based actor systems that handles the multithreading of different processes. (package timewarping)
3) Actix-web web API for external communication and control.(package webapi)


# Configuration
Configuration is based on the hocon config file, when starting the application the first argument should be the config file. Otherwise it defaults to config.conf.
The configuration can be accessed by `crate::SETTINGS` throughout the project.
Copy-pasting values that are not UTF-8 makes the library crash without good error messages.