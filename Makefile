ACT_IMAGE := catthehacker/ubuntu:act-latest
ARTIFACTS := dist/artifacts

DEPLOY_DIR := /mnt/d/tmp/jtb

.PHONY: build deploy
build:
	act pull_request -W .github/workflows/build.yml -P ubuntu-latest=$(ACT_IMAGE) --artifact-server-path $(ARTIFACTS)

# Copies the built exe to D:\tmp\jtb; it is launched by hand on Windows.
deploy:
	unzip -o $(ARTIFACTS)/*/jtb-app/jtb-app.zip -d dist
	mkdir -p $(DEPLOY_DIR)
	rm -f $(DEPLOY_DIR)/*.cmd
	cp dist/jtb-app.exe scripts/*.cmd scripts/jtb.json $(DEPLOY_DIR)/
