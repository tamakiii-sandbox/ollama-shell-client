.PHONY: help setup teardown activate

help:
	@cat $(firstword $(MAKEFILE_LIST))

setup: .venv

teardown:
	rm -rf .venv

activate:
	$(error "run `source .venv/bin/activate`")

.venv:
	python -m poetry install

