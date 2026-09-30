# tentzhen.com

Plain HTML and CSS, no build step. To look at it, serve it on this machine only:

```fish
python3 -m http.server 8000 --bind 127.0.0.1 -d site
open http://127.0.0.1:8000/
```

Opening the files directly (`open site/index.html`) left the build pages unstyled in Safari:
they load `style.css` from three folders up, and over `file://` it did not load. Over http it
does. Keep `--bind 127.0.0.1`; without it the server listens on every network interface.

- **Desktop first.** The rail down the left and the skyline at the bottom are laid out for a
  column of about 1440 px. Narrow screens are not designed yet.
- **`builds/` is generated** from the records in `builds/` and `parts/` at the repo root by
  `cargo run -p tentzhen-site`. Never edit it by hand: the Quality Gate regenerates it and fails
  on any difference.
- **Images** in `img/` are copies of files in `brand/`. The Quality Gate fails if a copy drifts,
  so change the file in `brand/` and copy it over.
- **Fonts** are served from `fonts/`, with their licences; `fonts/README.md` says where each came
  from. A page loads nothing from another site.
- **Hosting:** GitHub Pages publishes this directory, as it is on main, once main's Quality Gate
  has passed on that commit (`.github/workflows/pages.yml`). It is served at tentzhen.com once
  the domain's DNS points at GitHub.
