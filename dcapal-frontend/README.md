# DcaPal Frontend

Set up the [local Supabase environment](https://supabase.com/docs/guides/local-development):

```shell
make supabase-up
```

Once the docker instance is started, copy the displayed anon key and replace the `VITE_SUPABASE_ANON_KEY` in the
`.env`
file.

```dotenv
REACT_APP_ENABLE_COOKIE_BUTTON=0
VITE_SUPABASE_URL=http://127.0.0.1:54321
VITE_SUPABASE_ANON_KEY=<anon_key>
```

Run the frontend server

```shell
cd dcapal/dcapal-frontend
npm run start
```
