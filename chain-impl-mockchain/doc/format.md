**This is a draft document**

1. [Preliminaries](#preliminaries)
1. [Block](#block)
    1. [Block Header](#block-header)
    1. [Block Content](#block-content)
        1. [Fragment: Initial](#fragment-initial)
        1. [Fragment: Old UTxO Declaration](#fragment-old-utxo-declaration)
        1. [Fragment: Others](#fragment-others)
            1. [Common Structure](#common-structure)
            1. [Payload](#payload)
            1. [Inputs/Outputs](#inputsoutputs)
            1. [Witnesses](#witnesses)

# Preliminaries

1. In the following notations, we use `|` to represent concatenation of two sequences of bytes and `||` to represent an alternative choice between two paths.

2. All integers are encoded in **big-endian** format.

3. All signatures have the format:

```
SIGNATURE = SIGNATURE-SIZE (2 bytes) | SIGNATURE-PAYLOAD (SIGNATURE-SIZE bytes)
```

4. In the formats below, `H` refers to a Blake2b (256 bits) hashing algorithm.


# Block

```
BLOCK = BLOCK-HEADER | BLOCK-CONTENT
```

## Block Header

The header is a small piece of data, containing enough informations for validation and network deduplication and a strong signed cryptographic link to the content.

```
BLOCK-HEADER = BLOCK-HEADER-COMMON | (BLOCK-HEADER-BFT || BLOCK-HEADER-PRAOS)

BLOCK-HEADER-COMMON 
  = BLOCK-HEADER-SIZE (2 bytes) 
  | BLOCK-HEADER-VERSION (2 bytes) 
  | BLOCK-HEADER-CONTENT-SIZE (4 bytes)
  | BLOCK-HEADER-DATE 
  | BLOCK-HEADER-CHAIN-LENGTH (4 bytes)
  | BLOCK-HEADER-CONTENT-HASH 
  | BLOCK-HEADER-PARENT-HEADER-HASH (32 bytes)

BLOCK-HEADER-DATE = BLOCK-HEADER-EPOCH (4 bytes) | BLOCK-HEADER-SLOT (4 bytes)

BLOCK-HEADER-CONTENT-HASH = H(BLOCK-CONTENT) (32 bytes)

BLOCK-HEADER-BFT = BFT-LEADER-PUBKEY (32 bytes) | BFT-SIGNATURE (64 bytes)

BLOCK-HEADER-PRAOS 
  = PRAOS-VRF-PUBKEY (32 bytes) 
  | PRAOS-VRF-PROOF (96 bytes)
  | PRAOS-KES-SIGNATURE (484 bytes)
```

##### Remarks

1. The `BLOCK-HEADER-COMMON` part is 84 bytes in total

2. Maximum header is thus 64K not including the block content

3. First block has chain length 0

4. We reserved the special value of all 0 for the parent header hash,
   to represent the lack of parent for the block0, but for other blocks
   it's not reserved and could represent, although with negligeable
   probability, a valid block. In any case, it means that there's no 
   special meaning to this value in normal context.  

Additionally, we introduce the capability to address each header individually
by using a cryptographic hash function `H`. The hash include all
the content serialized in the sequence above, except the size of header,
which effectively means that calculating the hash of a fully serialized
header is just applying the hash function to the binary data except the first
2 bytes.

```
BLOCK-HEADER-ID = H(BLOCK-HEADER[2..])
```

## Block Content

We need to be able to have different type of content on the blockchain, we also
need a flexible system for future expansion of this content.  The block content
is effectively a sequence of serialized content, one after another.

Each individual piece of block content is called a **Fragment** and is prefixed
with a header which contains the following information:

```
FRAGMENT-HEADER = SIZE (2 bytes) | FRAGMENT-TYPE (1 byte) 

FRAGMENT-TYPE 
  = FRAGMENT-TYPE-INITIAL (= 0x00)
  | FRAGMENT-TYPE-OLD-UTXO-DECLARATION (= 0x01)
  | FRAGMENT-TYPE-TRANSACTON (= 0x02)
  | FRAGMENT-TYPE-OWNER-STAKE-DELEGATION (= 0x03)
  | FRAGMENT-TYPE-STAKE-DELEGATION (= 0x04)
  | FRAGMENT-TYPE-POOL-REGISTRATION (= 0x05)
  | FRAGMENT-TYPE-POOL-MANAGEMENT (= 0x06)
  | FRAGMENT-TYPE-UPDATE-PROPOSAL (= 0x07)
  | FRAGMENT-TYPE-UPDATE-VOTE (= 0x08)
```

The block body is formed of the following stream of data:

```
BLOCK-CONTENT = (FRAGMENT-HEADER | FRAGMENT{FRAGMENT-TYPE})*
```

Additionally, we introduce the capability to refer to each fragment
individually by FragmentId, using a cryptographic hash function :

```
FRAGMENT-ID = H(FRAGMENT-TYPE | FRAGMENT{FRAGMENT-TYPE})
```

The hash doesn't include the size prefix in the header to simplify
calculation of hash with on-the-fly (non serialized) structure.

### Fragment: Initial

This message type may only appear in the genesis block (block 0) and
specifies various configuration parameters of the blockchain. Some of
these are immutable, while other may be changed via the update
mechanism (see below). The format of this message is:

```
                                <------------- N times ----------->
FRAGMENT{INITIAL} = N (2 bytes) | CONFIG-PARAM | ... | CONFIG-PARAM

CONFIG-PARAM = CONFIG-PARAM-TAGLEN (2 bytes) | CONFIG-PARAM-CONTENT
```

##### Remarks

`CONFIG-PARAM-TAGLEN` is a 16-bit bitfield that has the size 
of the payload (i.e. the value of the parameter) in bytes in the 6 least-significant
bits, and the type of the parameter in the 10 most-significant bits. Note that
this means that the payload cannot be longer than 63 bytes.

```      
                      <------------ 16 bits -------->
CONFIG-PARAM-TAGLEN = t.t.t.t.t.t.t.t.t.t.s.s.s.s.s.s 
                      <------ type -----> <-- size ->
```

The following config parameter types exist:

| tag  | name                                 | content type | description                                                                            |
| :--- | :----------------------------------- | :----------- | :------------------------------------------------------------------------------------- |
| 1    | discrimination                       | u8           | address discrimination; 1 for production, 2 for testing                                |
| 2    | block0-date                          | u64          | the official start time of the blockchain, in seconds since the Unix epoch             |
| 3    | consensus                            | u16          | consensus version; 1 for BFT, 2 for Genesis Praos                                      |
| 4    | slots-per-epoch                      | u32          | number of slots in an epoch                                                            |
| 5    | slot-duration                        | u8           | slot duration in seconds                                                               |
| 6    | epoch-stability-depth                | u32          | the length of the suffix of the chain (in blocks) considered unstable                  |
| 8    | genesis-praos-param-f                | Milli        | determines maximum probability of a stakeholder being elected as leader in a slot      |
| 9    | max-number-of-transactions-per-block | u32          | maximum number of transactions in a block                                              |
| 10   | bft-slots-ratio                      | Milli        | fraction of blocks to be created by BFT leaders                                        |
| 11   | add-bft-leader                       | 32 bytes     | add a BFT leader                                                                       |
| 12   | remove-bft-leader                    | 32 bytes     | remove a BFT leader                                                                    |
| 13   | allow-account-creation               | bool (u8)    | 0 to enable account creation, 1 to disable                                             |
| 14   | linear-fee                           | LINEAR-FEE   | coefficients for fee calculations                                                      |
| 15   | proposal-expiration                  | u32          | number of epochs until an update proposal expires                                      |
| 16   | kes-update-speed                     | u32          | maximum number of seconds per update for KES keys known by the system after start time |

##### Remarks

1. `Milli` is a 64-bit entity that encoded a non-negative, fixed-point number with a scaling factor of 1000. That is, the number 1.234 is represented as the 64-bit unsigned integer 1234.

2. `LINEAR-FEE` has the following format:

   ```
   LINEAR-FEE = FEE-CONSTANT (4 bytes) | FEE-COEFFICIENT (4 bytes) | FEE-CERTIFICATE (4 bytes)
   ```

### Fragment: Old UTxO Declaration

```
                                            <--------------------- N times ------------------->
FRAGMENT{OLD-UTXO-DECLARATION} = N (1 byte) | OLD-UTXO-DECLARATION | ... | OLD-UTXO-DECLARATION

OLD-UTXO-DECLARATION = COIN (8 bytes) | LEGACY-ADDRESS

LEGACY-ADDRESS = LEGACY-ADDRESS-SIZE (2 bytes) | LEGACY-ADDRESS-PAYLOAD (LEGACY-ADDRESS-SIZE bytes)
```

### Fragment: Others

#### Common Structure

Fragment contents unless otherwise specify are in the following generic format:

```
FRAGMENT = PAYLOAD{FRAGMENT-TYPE} | IOS | WITNESSES
```

`PAYLOAD` can be empty depending on the specific message. `WITNESSES` allows
binding the `PAYLOAD` with the Witness to prevent replayability when necessary, and
its actual content is linked to the `PAYLOAD`.

This construction is generic and allow payments to occurs for either transfer of value
and/or fees payment, whilst preventing replays.

#### Payload

Here below are the known `PAYLOAD` formats for various fragment types:

```
PAYLOAD{INITIAL} = N/A

PAYLOAD{OLD-UTXO-DECLARATION} = N/A

PAYLOAD{TRANSACTION} = Ø (0 byte)

PAYLOAD{OWNER-STAKE-DELEGATION} = TODO

PAYLOAD{STAKE-DELEGATION} = ACCOUNT_PUBKEY (32 bytes) | PRAOS-VRF-PUBKEY (32 bytes)

PAYLOAD{POOL-REGISTRATION} = TODO

PAYLOAD{POOL-MANAGEMENT} = TODO

PAYLOAD{UPDATE-PROPOSAL} = PROPOSAL | PROPOSER-ID (32 bytes) | PROPOSER-SIGNATURE (?? bytes)

PAYLOAD-TYPE-UPDATE-VOTE = PROPOSAL-ID (32 bytes) | VOTER-ID (32 bytes) | VOTER-SIGNATURE (?? bytes)

PROPOSAL = FRAGMENT{INITIAL}
```

##### Remarks

1. `PROPOSER-ID` is a ed25519 extended public key.

2. `PROPOSER-SIGNATURE` is a signature by the corresponding private key over the string `PROPOSAL | PROPOSER-ID`.

3. `PROPOSAL-ID` is a `FRAGMENT-ID` corresponding to a previous `FRAGMENT{UPDATE-PROPOSAL}`

4. `VOTER-ID` is an ed25519 extended public key.

5. `VOTER-SIGNATURE` is a signature by the corresponding secret key over `PROPOSAL-ID | VOTER-ID`.

#### Inputs/Outputs

Inputs/Outputs is in the following format:

```
                                        <--- N-INPS times --->
IOS = N-INPS (1 byte) | N-OUTS (1 byte) | INPUT | .. | INPUT | OUTPUT | .. | OUTPUT 
                                                             <--- N-OUTS times --->

INPUT = INPUT-INDEX (1 byte) | INPUT-VALUE (8 bytes) | INPUT-REFERENCE 

INPUT-REFERENCE = ACCOUNT-PUBKEY (32 bytes) || FRAGMENT-ID (32 bytes)

OUTPUT = OUTPUT-ADDRESS (33 or 65 bytes) | OUTPUT-VALUE (8 bytes)
```

##### Remarks

1. 256 inputs maximum.

2. 255 outputs maximum, `0xff` is reserved. 

3. All `INPUT` are therefore `41` bytes.

4. A special `INPUT-INDEX` value of `0xff` specifies an account spending.

#### Witnesses

To authenticate the `PAYLOAD` and the IOs, we add witnesses with a 1-to-1 mapping
with inputs. The serialized sequence of inputs, is directly linked with the
serialized sequence of witnesses.

Fundamentally the witness is about signing a message and generating/revealing
cryptographic material to approve unequivocally the content.

There's currently 3 differents types of witness supported:

1. Old utxo scheme: an extended public key, followed by a ED25519 signature
1. Utxo scheme: a ED25519 signature
1. Account scheme: a counter and an ED25519 signature

With the following serialization:

```
            <----------------- N-INPS times ----------------->
WITNESSES = WITNESS{WITNESS-TYPE} | .. | WITNESS{WITNESS-TYPE}

WITNESS{OLD-UTXO} = WITNESS-TYPE (1 byte) | EXTENDED-PUBKEY (64 bytes) | WITNESS-SIGNATURE (64 bytes)

WITNESS{UTXO} = WITNESS-TYPE (1 byte) | WITNESS-SIGNATURE (64 bytes)

WITNESS{ACCOUNT} = WITNESS-TYPE (1 byte) | WITNESS-SIGNATURE (64 bytes)

WITNESS-TYPE 
  = WITNESS-TYPE-OLD-UTXO (= 0x00)
  | WITNESS-TYPE-UTXO (= 0x01)
  | WITNESS-TYPE-ACCOUNT (= 0x02)
```

The signing message for witnesses, w.r.t the cryptographic Ed25519 signature, is generally of the form:

```
WITNESS-SIGNATURE = S(WITNESS-SIGNATURE-DATA)

WITNESS-SIGNATURE-DATA = H(GENESIS-BLOCK-HEADER) | H(PAYLOAD{FRAGMENT-TYPE} | IOS) | WITNESS-SPECIFIC-DATA
```

More specifically, for `TRANSACTION` we do have:

```
WITNESS-SIGNATURE-DATA-OLD-UTXO = H(GENESIS-BLOCK-HEADER) | H(IOS)

WITNESS-SIGNATURE-DATA-UTXO = H(GENESIS-BLOCK-HEADER) | H(IOS)

WITNESS-SIGNATURE-DATA-ACCOUNT = H(GENESIS-BLOCK-HEADER) | H(IOS) | WITNESS-ACCOUNT-COUNTER (4 bytes) 
```

where `S` is an Ed25519 signature using the private key associated with the corresponding input.

#### Rationale

- 1 byte index utxos: 256 utxos = 10496 bytes just for inputs, already quite 
  big and above a potential 8K soft limit for block content. Utxo representation
  optimisations (e.g. fixed sized bitmap).

- Values in inputs: Support for account spending: specifying exactly how much
  to spend from an account. Light client don't have to trust the utxo information
  from a source (which can lead to e.g. spending more in fees), since a client 
  will now sign a specific known value.

- Account Counter encoding:
  4 bytes: 2^32 unique spending from the same account is not really reachable:
  10 spending per second = 13 years to reach limit.
  2^32 signatures on the same signature key is stretching the limits of scheme.
  Just the publickey+witnesses for the maximum amount of spending would take 400 gigabytes

- Value are encoded as fixed size integer of 8 bytes, instead of using any sort 
  of VLE (Variable Length Encoding). While it does waste space for small values,
  it does this at the net advantages of simplifying handling from low memory 
  devices by not having need for a specific serialization format encoder/decoder
  and allowing value changing in binary format without having to reduce or grow 
  the binary representation.
