"""
Attribute names used by decorators and schema generation.

This module exists as a leaf dependency so both genlayer._internal.annotations
and genlayer._internal.get_schema can import these constants without circular imports.
"""

PUBLIC_ATTR = '__gl_public__'
READONLY_ATTR = '__gl_readonly__'
MIN_GAS_LEADER_ATTR = '__gl_min_gas_leader__'
MIN_GAS_VALIDATOR_ATTR = '__gl_min_gas_validator__'
PAYABLE_ATTR = '__gl_payable__'
