# Fonts

The site's three fonts, served from here so a page loads nothing from anyone else. Fetched
2026-09-30. Each is under the SIL Open Font License 1.1, whose text is beside it.

- **Chakra Petch** (`OFL-chakra-petch.txt`): Google Fonts' latin and latin-ext subsets, as its CSS
  served them. The licence reserves no font name, so a subset may keep it.
- **IBM Plex Sans and IBM Plex Mono** (`OFL-ibm-plex-sans.txt`, `OFL-ibm-plex-mono.txt`): IBM's own
  complete files, unmodified. The licence reserves the name "Plex", which a modified file, such as
  a subset, may not use, so these are not subset.

| file | from | sha256 |
|---|---|---|
| `chakra-petch-500-latin.woff2` | fonts.gstatic.com, Chakra Petch v13, weight 500, latin subset | `adf007cd277ef6dac0c8cecacfb60cc5e24e7c8bcce4db98aa233bd3a88ef5a3` |
| `chakra-petch-500-latin-ext.woff2` | fonts.gstatic.com, Chakra Petch v13, weight 500, latin-ext subset | `b821fcd55102d303590e2b5da0c6097f2e0c384bd5ad8fbf157bb5e91402c250` |
| `chakra-petch-600-latin.woff2` | fonts.gstatic.com, Chakra Petch v13, weight 600, latin subset | `4d6d5f0b31b3a471a843779c4ecada040e5bc291f96bca9805a94293fe8003fa` |
| `chakra-petch-600-latin-ext.woff2` | fonts.gstatic.com, Chakra Petch v13, weight 600, latin-ext subset | `140bd5c457550ed1e06cf2346e5ab432d4a7344c4ba44d5460ee7384f210d098` |
| `IBMPlexSans-Light.woff2` | npm @ibm/plex-sans 1.1.0, fonts/complete/woff2 | `769209c2a0dbf2e3f012c22e4c604100cb3f1e7b8beb0ef77bc7d982d85509cc` |
| `IBMPlexSans-Regular.woff2` | npm @ibm/plex-sans 1.1.0, fonts/complete/woff2 | `ba711a3085ff9f27440b6b9c4550cfc47c97bf36591d5da958b975bb3add8c1a` |
| `IBMPlexSans-Medium.woff2` | npm @ibm/plex-sans 1.1.0, fonts/complete/woff2 | `5660f8a658f8bb50dbc005232f885eadffd2bc1c235c4f6fbb63469d1f9cde6d` |
| `IBMPlexMono-Regular.woff2` | npm @ibm/plex-mono 2.5.0, fonts/complete/woff2 | `ba204497f16b6d334cee9d1e963a831b73e3a56e1d6300a8489d18df7214b350` |
| `IBMPlexMono-Medium.woff2` | npm @ibm/plex-mono 2.5.0, fonts/complete/woff2 | `33faf307fa6031fb4062276d7320a6d632de890cbb347576fd80cfa01077bc25` |

`../style.css` declares each with `@font-face`. To change a font, replace its file, update this
table, and keep its licence beside it.
