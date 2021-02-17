# Backend API to be hosted at api.worldrank.org

Make sure that the `path_to_data` directory specified in [configuration/base.yml](configuration/base.yml) contains `all_players.csv`, `all_contests.csv`, and all the files `players/{handle}.csv`. See the parent directory for the command that creates these.

After [installing Rust](https://www.rust-lang.org/tools/install), build and run the program with:
```
cargo run --release
```

Now you can access the API from another terminal or application. Examples:
```
curl http://127.0.0.1:8000/top -d "start=0&many=10"
curl http://127.0.0.1:8000/count -d ""
curl http://127.0.0.1:8000/count -d "min=1400&max=1599"
curl http://127.0.0.1:8000/player -d "handle=EbTech"
```

If you'd like to learn how to make an application like this, check out Luca Palmieri's excellent blog and book at [zero2prod.com](https://zero2prod.com)!
