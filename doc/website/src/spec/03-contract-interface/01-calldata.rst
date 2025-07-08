Calldata Format
===============

Calldata is a format used within GenVM to exchange data between
contracts and VMs. It is designed to be safe to load, dynamically typed
and JSON-like, binary and compact, and supports blockchain specific
types.

Types
-----

Calldata is one of:

1. Arbitrary big integer
2. Raw bytes
3. UTF-8 string
4. Array of Calldata
5. Mapping from strings to Calldata
6. Address (20 bytes)

Format
------

ULEB128 Encoding
~~~~~~~~~~~~~~~~

“Unsigned little endian base 128” is a variable-length code compression
used to store arbitrarily large integers.

**Encoding**: Split number into groups of 7 bits, little-endian, zero
extend the biggest one. For each except the biggest one (rightmost), set
8th bit to one and concatenate.

**Examples**: - 0 ↔ 0x00 - 1 ↔ 0x01 - 128 ↔ 0x80 0x01

Calldata Encoding
~~~~~~~~~~~~~~~~~

Each calldata value starts with a ULEB128 number, which is treated as
follows:

+------------------------+------------------+-----------------------------+-----------------------------------------------+
|least significant 3 bits| interpreted as   |number shifted by this 3 bits|followed by                                    |
|                        | type             |                             |                                               |
+========================+==================+=============================+===============================================+
|0                       |atom              |0 ⇒ ``null``                 |nothing                                        |
|                        |                  |                             |                                               |
|                        |                  |1 ⇒ ``false``                |nothing                                        |
|                        |                  |                             |                                               |
|                        |                  |2 ⇒ ``true``                 |nothing                                        |
|                        |                  |                             |                                               |
|                        |                  |3 ⇒ followed by address      |20 bytes of address                            |
|                        |                  |                             |                                               |
|                        |                  |_ ⇒ reserved for future use  |reserved for future use                        |
|                        |                  |                             |                                               |
|                        |                  |                             |                                               |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|1                       |positive int  or 0|``value``                    | nothing                                       |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|2                       |negative int      |``abs(value) - 1``           | nothing                                       |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|3                       |bytes             |``length``                   |``bytes[length]``                              |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|4                       |string            |``length``                   |``bytes[length]`` of utf8 encoded string       |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|5                       |array             |``length``                   |``calldata[length]``                           |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|6                       |map               |``length``                   |``Pair(FastString, calldata)[length]``         |
|                        |                  |                             | sorted by keys                                |
+------------------------+------------------+-----------------------------+-----------------------------------------------+
|7                       |reserved for      |                             |                                               |
|                        |future use        |                             |                                               |
+------------------------+------------------+-----------------------------+-----------------------------------------------+


**FastString** is encoded as ULEB128 length followed by UTF-8 encoded
bytes (difference is that it does not have a type).
