# instantly ⚡

![version](https://img.shields.io/crates/v/instantly?style=flat-square)
![license](https://img.shields.io/crates/l/instantly?style=flat-square)

Download photos from instagram with ease.

To get started, simply:

```bash
cargo install instantly
instantly --help
```

<p align="center" style="padding: 32px 0 32px 0;">
<a href="https://asciinema.org/a/paEAWYUIcfv5OH1egzlHAmwOf"><img src="https://asciinema.org/a/paEAWYUIcfv5OH1egzlHAmwOf.png" width="512"/></a>
</p>

## How do I fetch multiple pages? What is a 'query hash'?

The query hash is used by instagram to validate internal
api calls. You only need to do this once, and can reuse it.

- open up instagram.com
- visit any user
- open up the inspector
- trigger an xhr request
- extract the `query_hash` id from the query string
