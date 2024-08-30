class SameOp:
    def __init__(self, l, r):
        self.l = l
        self.r = r

    def __call__(self, foo, *, void=False):
        threwl = False
        threwr = False
        resl = None
        resr = None
        try:
            resl = foo(self.l)
        except:
            threwl = True
        try:
            resr = foo(self.r)
        except:
            threwr = True
        assert threwl == threwr
        if threwl:
            return
        if void:
            resl = None
            resr = None
        assert resl == resr
