![rememberall-logo](https://s3.amazonaws.com/nicholasyager.com/assets/2016-09-17/rememberall.png)
# Rememberall

Rememberall is a command line document retreval system that returns relevent
documents based on word stemming and Bayesian statistics.

## Quick Start
To install:
```
cargo install
```

To index a directory of documents
```
rememberall index path/doc/documents/*
```

To query the index
```
rememberall search keyword1 keyword2 ...
```

## Demo
[![asciicast](https://asciinema.org/a/86108.png)](https://asciinema.org/a/86108)

## Suggested Document Format
This tool is best suited for files following a markdown-based format, in which
document titles are top-level headers. For example:
```
# Document Title

This is an example document that will work well with remeberall. Let's keep it
simple where possible, man!
```

## References:
Remembrance Agent: A continuously running automated information retrieval system - http://alumni.media.mit.edu/~rhodes/Papers/remembrance.html

---
Rememberall - More magical than the original.
