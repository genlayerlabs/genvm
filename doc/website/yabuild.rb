POETRY_RUN = ['poetry', 'run', '-C', cur_src]

docs_out = root_build.join('out', 'docs')
docs_out.mkpath

docs_out_txt = cur_build.join('txt')
docs_out_txt.mkpath

LIB_SRC = root_src.join('runners', 'genlayer-py-std', 'src')

codegen_src =  cur_src.parent.parent.join('executor', 'codegen')
