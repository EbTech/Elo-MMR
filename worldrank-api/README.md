# Backend API to be hosted at api.worldrank.org

Make sure that the `path_to_data` directory specified in [configuration/base.yml](configuration/base.yml) contains `all_players.csv` as well as the files `players/{handle}.csv`. After [installing Rust](https://www.rust-lang.org/tools/install), build and run the program with:
```
cargo run
```

Now you can access the API from another terminal or application. Examples:
```
curl -d "start=0&many=10" http://127.0.0.1:8000/top
curl -d "handle=EbTech" http://127.0.0.1:8000/player
```

If you'd like to learn how to make an application like this, check out Luca Palmieri's excellent blog and book at [zero2prod.com](https://zero2prod.com)!
