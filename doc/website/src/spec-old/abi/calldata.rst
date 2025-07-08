Calldata format
===============

Calldata is a format that is used within GenVM to exchange data between contracts and VMs. It is designed with following in mind:
- be safe to load
- be dynamically typed and json-like
- be binary and compact
- support blockchain specific types

Types
-----
*Calldata* is one of:

#. arbitrary big integer
#. raw bytes
#. utf8 string
#. array of *Calldata*
#. mapping from strings to *Calldata*
#. Address (20 bytes)

Format
------

"uleb128"
^^^^^^^^^
"unsigned little endian base 128" is a variable-length code compression used to store arbitrarily large integers

Encoding: split number into groups of 7 bits, little-endian, zero extend the biggest one. For each except the biggest one (rightmost), set 8th bit to one and concatenate

Examples:

* 0 ↔ 0x00
* 1 ↔ 0x01
* 128 ↔ 0x80 0x01

Calldata
^^^^^^^^

Each calldata value starts with uleb128 number, which is treated as follows:

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

``FastString`` is encoded as uleb128 length followed by utf8 encoded bytes (difference is that it does not have a type)
