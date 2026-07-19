# Web dashboard index

Description: Navigation map for the Vite/React marketing site, replay, and observability dashboard.
Purpose: Identify UI ownership and the boundary for connecting the dashboard to the Rust API.

## Entry points and components

- `src/main.jsx` — mounts the React application.
- `src/App.jsx` — hash-based routing; `#/app` opens the dashboard and other routes open the landing page.
- `src/Landing.jsx` — marketing page and replay composition.
- `src/ToolReplay.jsx` — live animated tool-call replay and token visualization.
- `src/Dashboard.jsx` — live call timeline, original/intercepted comparison, and context-window overlay.
- `src/api.js` — fetches the current session and hydrates tool-call summaries with detail payloads.
- `src/useLiveSession.js` — polls the backend API for React consumers.
- `src/Antigravity.jsx` — React Three Fiber hero visualization.
- `src/index.css` — shared application styling.
- `vite.config.js` — Vite configuration and development proxy to the gateway API.
- `eslint.config.js` — lightweight JavaScript and JSX lint configuration.
- `package.json` — frontend dependencies and `dev`, `lint`, `build`, and `preview` scripts.

## Backend integration boundary

- `ToolReplay` and `Dashboard` consume the current live gateway session.
- `src/api.js` fetches call details because the current-session endpoint only returns lightweight summaries.
- `../server/src/api.rs` defines `GET /health`, `GET /api/v1/sessions/current`, and `GET /api/v1/tool-calls/{id}`.
- `../server/src/schema.rs` is the authoritative camelCase response contract.
- The current-session endpoint returns lightweight call summaries. Fetch tool-call detail separately for original and intercepted payload text.
- The backend defaults to `127.0.0.1:8787`; browser integration needs either a Vite development proxy or backend CORS support.

No frontend test setup currently exists.

## Related indexes

- `../INDEX.md` — repository and backend navigation.
