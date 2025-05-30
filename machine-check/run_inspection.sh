if [ "$1" = "--short" ]; then
	cargo test -- --ignored --nocapture --exact short_inspection 2>&1 | grep --line-buffered "done-special-symbol" | pv -N "Short inspection" -l -t -p -s 4 >> /dev/null
	exit 0
fi
cargo test -- --ignored --nocapture --exact full_inspection 2>&1 | grep --line-buffered "done-special-symbol" | pv -N "Full inspection" -l -t -p -s 454 >> /dev/null

