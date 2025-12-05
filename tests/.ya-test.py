import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()

local_ctx.eval_file('plugins/cargo.py')
local_ctx.eval_file('plugins/pytest.py')
