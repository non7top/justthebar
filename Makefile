ACT_IMAGE := catthehacker/ubuntu:act-latest
ARTIFACTS := dist/artifacts

DEPLOY_DIR := /mnt/d/tmp/jtb

.PHONY: build deploy fmt fmt-image
build:
	act pull_request -W .github/workflows/build.yml -P ubuntu-latest=$(ACT_IMAGE) --artifact-server-path $(ARTIFACTS)

# Formats the workspace in a throwaway container, as the invoking user.
fmt-image:
	docker build -q -t jtb-fmt -f tools/fmt.Dockerfile tools

fmt: fmt-image
	docker run --rm --user "$$(id -u):$$(id -g)" -v "$(CURDIR)":/w -w /w jtb-fmt cargo fmt --all

# Copies the built exe and hook DLL to D:\tmp\jtb; it is launched by hand on Windows.
deploy:
	unzip -o $(ARTIFACTS)/*/jtb-app/jtb-app.zip -d dist
	mkdir -p $(DEPLOY_DIR)
	rm -f $(DEPLOY_DIR)/*.cmd
	cp dist/jtb-app.exe dist/jtb_hook.dll scripts/*.cmd scripts/jtb.json $(DEPLOY_DIR)/
