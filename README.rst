browsercookie-rs
################

A rust crate useful for extracting cookies from browsers. Inspired from
`browsercookie <https://pypi.org/project/browsercookie/>`_ python library.

Library
*******

Usage
=====

Using the library is quite simple

.. code-block:: rust

        // Cargo.toml
        [dependencies]
        browsercookie-rs = "0.1.1"

.. code-block:: rust

        use browsercookie::{CookieFinder, Browser, Attribute};

        let mut cookie_jar = CookieFinder::builder()
            .with_regexp(Regex::new("google.com").unwrap(), Attribute::Domain)
            .with_browser(Browser::Firefox)
            .build
            .find()
            .await.unwrap();

        let cookie = cookie_jar.get("some_cookie_name").unwrap();

        println!("Cookie header string: Cookie: {}", cookie);

Better example should be present in `browsercookies <src/bin.rs>`_.

Binary
******

Same crate should also give you a binary ``browsercookies``, which should be usable
from your favourite shell for crudely using frontend apis for simple tooling.

.. code-block:: rust

        browsercookies --domain jira

Install
=======

.. code-block:: bash

        cargo install -f browsercookie-rs


Feature Matrix
==============

========== ========= ========
TargetOS    Firefox   Chrome
========== ========= ========
Linux          ✔        ✗
macOS          ✔        ✗
Windows        ✗        ✗
========== ========= ========

