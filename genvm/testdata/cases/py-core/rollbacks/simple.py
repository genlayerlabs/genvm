# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.public
def main():
    gsdk.rollback_immediate("nah, I won't execute")
