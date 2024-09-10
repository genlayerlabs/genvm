# Finding installed libraries

Some languages require more files or code to be ran. For instance Python requires an interpreter, C can have a dynamically linked libc and so on. To address contract size issue genvm introduces a concept of runners

Each runner is identified with `<human-readable-id>:<hash>` and is a `.zip` file, see [description section](#processing-of-runner-zip)

Then contract is one of the following:
1. wasm file, then it is linked and ran as-is, without any additional steps
2. zip file, then it is [processed as a runner `.zip`](#processing-of-runner-zip)
3. text file starting with a comment (as of now, `#`, `//`, `--` are supported), then it's comment is treated as `runner.json` from [runner `.zip`](#processing-of-runner-zip). Note, that in this scenario `actions` are not supported, as they can't start `wasm` --- the only entrypoint of a program

# Processing of runner `.zip`
This file must contain `runner.json` conforming to [schema](./runner-schema.json) and other arbitrary files. This file is processed as follows:
- `pre_actions` are applied
- `depends` are loaded and applied using same schema, note that same dependency won't be loaded twice
- `actions` are applied
