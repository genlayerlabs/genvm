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

* 0 <-> 0x00
* 1 <-> 0x01
* 128 <-> 0x00 0x81

Calldata
^^^^^^^^

Each calldata value starts with uleb128 number, which is treated as follows:

+------------------+------------------------+-----------------------------+-----------------------------------------------+
| type             |least significant 3 bits|number shifted by this 3 bits|followed by                                    |
+==================+========================+=============================+===============================================+
|special           |0                       |0 ⇒ null                     |nothing                                        |
|                  |                        |                             |                                               |
|                  |                        |1 ⇒ false                    |nothing                                        |
|                  |                        |                             |                                               |
|                  |                        |2 ⇒ true                     |nothing                                        |
|                  |                        |                             |                                               |
|                  |                        |3 ⇒ address                  |20 bytes of address                            |
|                  |                        |                             |                                               |
|                  |                        |_ ⇒ reserved for future use  |reserved for future use                        |
|                  |                        |                             |                                               |
|                  |                        |                             |                                               |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|positive int  or 0|1                       |``value``                    | nothing                                       |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|negative int      |2                       |``abs(value) - 1``           | nothing                                       |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|bytes             |3                       |``length``                   |``bytes[length]``                              |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|string            |4                       |``length``                   |``bytes[length]`` of utf8 encoded string       |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|array             |5                       |``length``                   |``calldata[length]``                           |
+------------------+------------------------+-----------------------------+-----------------------------------------------+
|map               |6                       |``length``                   |``Pair(FastString, calldata)[length]``         |
|                  |                        |                             | sorted by keys                                |
+------------------+------------------------+-----------------------------+-----------------------------------------------+

``FastString`` is encoded as uleb128 length followed by utf8 encoded bytes (difference is that it does not have a type)
