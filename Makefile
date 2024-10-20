test:
	docker run -it --cap-add=all --rm -v "$$PWD":/usr/src/myapp -w /usr/src/myapp rust:1-slim-bullseye sh