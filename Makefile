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
	rm -f $(DEPLOY_DIR)/*.old* 2>/dev/null || true
	cp dist/jtb-app.exe $(DEPLOY_DIR)/ 2>/dev/null || (mv -f $(DEPLOY_DIR)/jtb-app.exe $(DEPLOY_DIR)/jtb-app.exe.old.$$ && cp dist/jtb-app.exe $(DEPLOY_DIR)/)
	cp dist/jtb_hook.dll $(DEPLOY_DIR)/ 2>/dev/null || (mv -f $(DEPLOY_DIR)/jtb_hook.dll $(DEPLOY_DIR)/jtb_hook.dll.old.$$ && cp dist/jtb_hook.dll $(DEPLOY_DIR)/)
	cp scripts/*.cmd scripts/jtb.json $(DEPLOY_DIR)/
