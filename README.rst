browsercookie-rs
################

A rust crate useful for extracting cookies from browsers. Inspired from
`browsercookie <https://pypi.org/project/browsercookie/>`_ python library.

Usage
=====

Using the library is quite simple

.. code-block:: rust

        use browsercookie::{Browsercookies, Browser};

        let mut bc = Browsercookies::new();
        let domain_regex = Regex::new("google.com").unwrap();

        bc.from_browser(Browser::Firefox, &domain_regex).expect("Failed to get cookies from firefox");
        println!("Cookie header string: Cookie: {}", bc.to_header(domain_regex));

Better example should be present in `browsercookies <src/bin.rs>`_.

Binary
======

Same crate should also give you a binary ``browsercookies``, which should be usable
from your favourite shell for crudely using frontend apis for simple tooling.

.. code-block:: rust

        browsercookies --domain jira


Feature Matrix
==============

========== ========= ========
TargetOS    Firefox   Chrome
========== ========= ========
Linux          ✔        ✗
macOS          ✔        ✗
Windows        ✗        ✗
========== ========= ========

