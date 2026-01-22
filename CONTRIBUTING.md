# Contributing

Thanks for your interest in helping out! The codebase is still in heavy flux and we’re not ready to accept contributions for a few months. It’s a bit of a mess right now, but we’d still love to hear your thoughts—please try it out and share any feedback you have.

## Access Contribution Checklist

- New auth logic goes in `access::auth`
- New DSL predicate maps to exactly one `access::*` function
- Macros only wire access rules; no access logic inside macro bodies
- Access code returns `AccessError`, never `canic::Error`
- `ic_cdk::trap` is forbidden outside lifecycle adapters (`crates/canic-core/src/lifecycle`)
