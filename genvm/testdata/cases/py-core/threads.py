# { "depends": ["genvm-rustpython:test"] }

from threading import Thread

__gsdk_self_run__ = True

def foo(x):
    for i in range(5):
        print('foo' + str(x))

threads = []
for i in range(5):
    thread = Thread(target=foo, args=(i,))
    thread.start()
    threads.append(thread)
for i in range(5):
    print("!!!")
for thread in threads:
    thread.join()
