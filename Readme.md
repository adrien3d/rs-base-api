# RS Base API
RS Base API is a boilerplate to quickstart your backend in Rust.
As of today, it provides basic user management routes and communicates with MongoDB.

# To run
```shell
cp .env-sample .env
# Edit .env file according to your setup
cargo run
```

# Roadmap

* Implement Sentry
* Send account management e-mails: AWS SES/Scaleway
* Make a generic store
* Handle another DB like [PostgreSQL](https://github.com/launchbadge/sqlx/blob/main/examples/postgres/json/src/main.rs)
* Develop a CLI to generate code interactively: clap
