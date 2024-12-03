# Running GenVM tests
As of now there are two main test suites, both of which require `./build-scripts/install/install-deps.rb --test`

### executor/testdata (mock tests)
Prerequisites:
- For `get_webpage` tests to work, one needs a compatible webdriver. There is a docker image for simplification: `./executor/testdata/web-container/run-test-docker.sh`
- For the default `exec_prompt` to work `OPENAIKEY` env variable should be set to openai key

Generic steps:
```bash
### install dependencies (once)
python3 -m pip install -r executor/testdata/runner/requirements.txt
sudo apt-get install -y wabt
### run
./executor/testdata/runner/run.py
```

### python tests (module tests)
```bash
### install python 3.12
sudo apt install libsqlite3-dev
# ^ without sqlite-dev further won't have sqlite3 required by tests
curl https://pyenv.run | bash
~/.pyenv/bin/pyenv install 3.12
### install poetry
python3 -m pip install poetry
poetry env use ~/.pyenv/versions/3.12.5/bin/python3.12
### install dependencies (once)
cd runners/genlayer-py-std
poetry install
### run tests
poetry run pytest
```
