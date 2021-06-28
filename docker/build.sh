


mkdir -p target
mkdir -p target/docker

cp docker/Dockerfile target/docker/
cp -r src target/docker/
cp -r config target/docker/
cp Cargo.lock target/docker/
cp Cargo.toml target/docker/
cp log4rs.yml target/docker/