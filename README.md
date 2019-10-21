Instructions to refresh ratings:
- python3 crawl_CF.py
- Add output contest numbers and updated count to add_contests.txt
- Update contest count in the assert of compute_ratings.rs
- rustc -O compute_ratings.rs
- ./compute_ratings
