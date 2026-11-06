# Visper website

The product homepage: a static, dependency-free single page (`index.html`, `style.css`, `main.js`). English by default, with a 中文 toggle (also picked automatically from the browser language, or forced with `?lang=zh`). Light and dark follow the system and use the app's own palette from `src/theme.rs`.

## Preview locally

```bash
cd site
python3 -m http.server 8080
```

Then open http://localhost:8080.

## Deploy on Vercel

Create a project with **Root Directory** `site` and the **Other** framework preset. There is no build step and no environment variables; `vercel.json` only adds a few headers.
