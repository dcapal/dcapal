# DcaPal Frontend

Follow
the [Supabase local development guide](https://supabase.com/docs/guides/local-development?queryGroups=package-manager&package-manager=npm)
to set up the Supabase environment.

Once the docker instance is started, copy the displayed anon key and replace the `VITE_SUPABASE_ANON_KEY` in the
`.env.development`
file.

```dotenv
REACT_APP_ENABLE_COOKIE_BUTTON=0
VITE_SUPABASE_URL=http://127.0.0.1:54321
VITE_SUPABASE_ANON_KEY=<anon_key>
```

Run the frontend server

```bash
cd dcapal/dcapal-frontend
npm run start
```
