for store_size in `ls configs/store_sizes/`; do
    for interleaving in `ls configs/interleaving`; do
        echo "cargo run --release -- -r sim configs/large.toml configs/ddr4.toml configs/store_sizes/${store_size} configs/interleaving/${interleaving}"
        cargo run --release -- -r sim configs/large.toml configs/ddr4.toml configs/store_sizes/${store_size} configs/interleaving/${interleaving}
    done
done