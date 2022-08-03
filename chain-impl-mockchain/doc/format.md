**This is a draft document**

# Preliminaries

All integers are encoded in big-endian format.

`Signature` has the format

    Length | Payload

where `Length` is a 16-bit unsigned integer `N`, and `Payload` is `N`
bytes of signature data.

# Block

Format is:

    Header | Content

## Block Header

The header is a small piece of data, containing enough informations for validation and network deduplication and a strong signed cryptographic link to the content.

Common (2 * 64 bits + 1 * 32 bits + 2 * 256 bits = 84 bytes):

* Size of Header: 2 bytes (16 bits): Maximum header is thus 64K not including the block content
* Version of block: 2 bytes (16 bits)
* Size of Content: 4 bytes (32 bits)
* Block Date: Epoch (4 bytes, 32 bits) + Slot-id (4 bytes - 32 bits)
* Chain length (number of ancestor blocks; first block has chain length 0): 4 bytes (32 bits)
* Hash of content `H(Content)` (32 bytes - 256 bits)
* Parent Header hash : 32 bytes (256 bits)

We reserved the special value of all 0 for the parent header hash, to
represent the lack of parent for the block0, but for other blocks it's not
reserved and could represent, although with negligeable probability, a valid
block. In any case, it means that there's no special meaning to this value in
normal context.

In BFT the header also contains (768 bits = 96 bytes):

* BFT Public Key of the leader (32 bytes)
* BFT Signature (64 bytes)

In Praos/Genesis the header also contains (612 bytes):

* VRF PubKey: 32 bytes (ristretto25519)
* VRF Proof: 96 bytes (ristretto25519 DLEQs)
* KES Signature: 484 bytes (sumed25519-12)

Additionally, we introduce the capability to address each header individually
by using a cryptographic hash function : `H(HEADER)`. The hash include all
the content serialized in the sequence above, except the size of header,
which effectively means that calculating the hash of a fully serialized
header is just applying the hash function to the binary data except the first
2 bytes.

## Block Body

We need to be able to have different type of content on the blockchain, we also
need a flexible system for future expansion of this content.  The block content
is effectively a sequence of serialized content, one after another.

Each individual piece of block content is called a fragment and is prefixed
with a header which contains the following information:

* Size of content piece in bytes (2 bytes)
* Type of piece (1 byte): up to 256 different type of block content.

The block body is formed of the following stream of data:

    HEADER(FRAGMENT1) | FRAGMENT1 | HEADER(FRAGMENT2) | FRAGMENT2 ...

Where HEADER is:

	SIZE (2 bytes) | TYPE (1 byte) | 00 (1 byte)

Additionally, we introduce the capability to refer to each fragment
individually by FragmentId, using a cryptographic hash function :

    FragmentId = H(TYPE | FRAGMENT-CONTENT)

The hash doesn't include the size prefix in the header to simplify
calculation of hash with on-the-fly (non serialized) structure.

Types of content:

* Transaction
* Old Transaction
* Owner stake Delegation
* Certificate (Staking, Pool, Delegation, ...)
* TBD Update
* TBD Debug Stats : block debug information in chain.

### Common Structure

Fragment contents unless otherwise specify are in the following generic format:

    1. PAYLOAD
    2. INPUTS/OUTPUTS
    3. WITNESSNES(using 1+2 as message)
    4. PAYLOAD-AUTHENTICATION(using 1+2+3 as message)

PAYLOAD can be empty depending on the specific message. PAYLOAD-AUTHENTICATION allows
binding the PAYLOAD with the Witness to prevent replayability when necessary, and
its actual content is linked to the PAYLOAD and can be empty too.

This construction is generic and allow payments to occurs for either transfer of value
and/or fees payment, whilst preventing replays.

#### Inputs/Outputs

Inputs/Outputs is in the following format:

    IOs = #INPUTS (1 byte) | #OUTPUTS (1 byte) | INPUT1 | .. | OUTPUT1 | ..

* Input number : 1 byte: 256 inputs maximum
* Output number : 1 byte where 0xff is reserved: 255 outputs maximum
* Transaction Inputs (Input number of time * 41 bytes):
  * Index (1 byte) : special value 0xff specify a account spending (single or multi)
  * Account Identifier or Utxo Identifier (also FragmentId) (32 bytes)
  * Value (8 bytes)
* Transaction Outputs (Output number of time):
  * Address (bootstrap address 33 bytes, delegation address 65 bytes, account address 33 bytes)
  * Value (8 bytes)

#### Witnesses

To authenticate the PAYLOAD and the IOs, we add witnesses with a 1-to-1 mapping
with inputs. The serialized sequence of inputs, is directly linked with the
serialized sequence of witnesses.

Fundamentally the witness is about signing a message and generating/revealing
cryptographic material to approve unequivocally the content.

There's currently 3 differents types of witness supported:

* Old utxo scheme: an extended public key, followed by a ED25519 signature
* utxo scheme: a ED25519 signature
* Account scheme: a counter and an ED25519 signature

With the following serialization:

* Type of witness: 1 byte
* Then either:
  * Type=1 Old utxo witness scheme (128 bytes):
    * ED25519-Extended Public key (64 bytes)
    * ED25519 Signature (64 bytes)
  * Type=2 utxo witness scheme (64 bytes):
    * ED25519 Signature (64 bytes)
  * Type=3 Account witness (68 bytes):
    * Account Counter (4 bytes : TODO-ENDIANNESS)
    * ED25519 Signature (64 bytes)

The message, w.r.t the cryptographic signature, is generally of the form:

    TRANSACTION-SIGN-DATA-HASH = H(PAYLOAD | IOs)
    Authenticated-Data = H(HEADER-GENESIS) | TRANSACTION-SIGN-DATA-HASH | WITNESS-SPECIFIC-DATA

#### Rationale

* 1 byte index utxos: 256 utxos = 10496 bytes just for inputs, already quite big and above a potential 8K soft limit for block content
Utxo representation optimisations (e.g. fixed sized bitmap)

* Values in inputs:
Support for account spending: specifying exactly how much to spend from an account.
Light client don't have to trust the utxo information from a source (which can lead to e.g. spending more in fees), since a client will now sign a specific known value.

* Account Counter encoding:
4 bytes: 2^32 unique spending from the same account is not really reachable:
10 spending per second = 13 years to reach limit.
2^32 signatures on the same signature key is stretching the limits of scheme.
Just the publickey+witnesses for the maximum amount of spending would take 400 gigabytes

* Value are encoded as fixed size integer of 8 bytes (TODO: specify endianness),
instead of using any sort of VLE (Variable Length Encoding). While it does
waste space for small values, it does this at the net advantages of
simplifying handling from low memory devices by not having need for a
specific serialization format encoder/decoder and allowing value changing in
binary format without having to reduce or grow the binary representation.
This

## Type 0: Initial blockchain configuration

This message type may only appear in the genesis block (block 0) and
specifies various configuration parameters of the blockchain. Some of
these are immutable, while other may be changed via the update
mechanism (see below). The format of this message is:

    ConfigParams

where `ConfigParams` consists of a 16-bit field denoting the number of
parameters, followed by those parameters:

    Length | ConfigParam*{Length}

`ConfigParam` has the format:

    TagLen Payload

where `TagLen` is a 16-bit bitfield that has the size of the payload
(i.e. the value of the parameter) in bytes in the 6 least-significant
bits, and the type of the parameter in the 12 most-significant
bits. Note that this means that the payload cannot be longer than 63
bytes.

The following parameter types exist:

| tag  | name                                 | value type | description                                                                            |
| :--- | :----------------------------------- | :--------- | :------------------------------------------------------------------------------------- |
| 1    | discrimination                       | u8         | address discrimination; 1 for production, 2 for testing                                |
| 2    | block0-date                          | u64        | the official start time of the blockchain, in seconds since the Unix epoch             |
| 3    | consensus                            | u16        | consensus version; 1 for BFT, 2 for Genesis Praos                                      |
| 4    | slots-per-epoch                      | u32        | number of slots in an epoch                                                            |
| 5    | slot-duration                        | u8         | slot duration in seconds                                                               |
| 6    | epoch-stability-depth                | u32        | the length of the suffix of the chain (in blocks) considered unstable                  |
| 8    | genesis-praos-param-f                | Milli      | determines maximum probability of a stakeholder being elected as leader in a slot      |
| 9    | max-number-of-transactions-per-block | u32        | maximum number of transactions in a block                                              |
| 10   | bft-slots-ratio                      | Milli      | fraction of blocks to be created by BFT leaders                                        |
| 11   | add-bft-leader                       | LeaderId   | add a BFT leader                                                                       |
| 12   | remove-bft-leader                    | LeaderId   | remove a BFT leader                                                                    |
| 13   | allow-account-creation               | bool (u8)  | 0 to enable account creation, 1 to disable                                             |
| 14   | linear-fee                           | LinearFee  | coefficients for fee calculations                                                      |
| 15   | proposal-expiration                  | u32        | number of epochs until an update proposal expires                                      |
| 16   | kes-update-speed                     | u32        | maximum number of seconds per update for KES keys known by the system after start time |

`Milli` is a 64-bit entity that encoded a non-negative, fixed-point
number with a scaling factor of 1000. That is, the number 1.234 is
represented as the 64-bit unsigned integer 1234.

`LinearFee` has the format:

    Constant | Coefficient | Certificate

all of them 64-bit unsigned integers, specifying how fees are computed
using the formula:

    Constant + Coefficient * (inputs + outputs) + Certificate * certificates

where `inputs`, `outputs` and `certificates` represent the size of the
serialization of the corresponding parts of a transaction in bytes.

## Type 2: Transaction

Transaction is the composition of the TokenTransfer structure followed directly by the witnesses. PAYLOAD needs to be empty. Effectively:

    TokenTransfer<PAYLOAD = ()> | Witnesses

TODO:

* Multisig
* Fees

## Type 2: OwnerStakeDelegation

    TokenTransfer<PAYLOAD = OwnerStakeDelegation> | Witnesses

    OwnerStakeDelegation = DelegationType

## Type 3: Certificate

Certificate is the composition of the TokenTransfer structure where PAYLOAD is the certificate data, and then the witnesses. Effectively:

    TokenTransfer<PAYLOAD = CERTIFICATE> | Witnesses

Known Certificate types:

* Staking declaration: declare a staking key + account public information
* Stake pool registration: declare the VRF/KES key for a node.
* Delegation: contains a link from staking to stake pool.

Content:

* PublicKey
* Signature of the witness with the private key associated to the revealed PublicKey

## Type 4: Update Proposal

Update proposal messages propose new values for blockchain
settings. These can subsequently be voted on. They have the following
form:

    Proposal | ProposerId | Signature

where `ProposerId` is a ed25519 extended public key, and `Signature`
is a signature by the corresponding private key over the string
`Proposal | ProposerId`.

`Proposal` has the following format:

    ConfigParams

where `ConfigParams` is defined above.

## Type 5: Update votes

Vote messages register a positive vote for an earlier update
proposal. They have the format

    ProposalId | VoterId | Signature

where `ProposalId` is the message ID of an earlier update proposal
message, `VoterId` is an ed25519 extended public key, and `Signature`
is a signature by the corresponding secret key over `ProposalId |
VoterId`.

## Type 11: Vote Cast

VoteCast message is used to vote for a particular voting event.

Full fragment representation in hex:
```
00000572000b93c52748d7f75ffb0267846af7b3ca5228d7a210908be3fc963f227ab7f039f81f0203bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b4979698590263054c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1abc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b4979698590263054c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1abc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305f0d6dd25bab8405561d5bb817d9b20236bd5bb88c21b73afc09818ce5c5e483302864143d55bdc29e4f86db8fc1fdee2e1e9cd7062e5e2fb5ca52f581fdb7cd75b5e7836744ecbac60802ca39e3d92ebdac06479ec27b410140488bcb73ff76f3bcaa135fe2f52ed3fcee49e7e44feb9948ab2ee242c052fb24fe03e74f5d31f4eda0263ac2b0fddb4b56a7210e892f306b2bec41a0e9f6d713980753418ca1a6f5e7d4ad2a52627d4f72d86757a052794d07798178b5e05068993222bf167af74d40b167e06b063ec6f7fdd7a907b27a030197c4d7e5b53e9828be208ba94e64ef286a900fe795a3219e1dc23a4b033ca300cbf350d48d655bcb6ce5de7bf007a2a0ee3305468ba753fca17f700d102e665115acfc0653c8636e06aee4688ed6ca2db9928191c9e6c930f9a2ebda82c58eb7481dee601905713a593f33ce23f4e0ca87a1b487a36544d10c6bffeb83c3e897f968cba066471dd0cfe540dfeee2a9ef2f8441c4cce519a90820147be22efbe06d9b20d72ed3fe3ffe4198dac020459a4e56d0f4beb687661c9c38777c4034c6dd0d5efbaa713df7971fbf23b1e0532c5813d8fdff7e4dd3ea3da8755ea457a0ca5681c555dccd915db9530c0f20aeba371fc09b1524dbef709694f1ef87d168392dfb9a17b23e05980436e2c4e0b5aa4653e54062da822e6849d45ffa8ad7b8d9e258124ecd5adb99c4986cd3907e7a24b5b93fe207cfe8d7ebf927f3932b314c649ec18b19958c9b5d63c44260921b79b3d5bd79021901e2a7f4de8ad42fd677b3f033cf1b0f2151cc221703105000000010000000005001d000000000000003433503f28035b5b470f3c38571e1748474e191c362c583057472941066031093e1e000000000000003d23541c2c601e5e281533631c284a2c093437044a0c07545e5505341b1e4722330700000000000000284f51171a2b5b0a2e4632452d125b3451170a004d182235390f484c2d1f2e22354300000000000000122a0e5d3608025f4b1b10281e2f2e252d2b56251734541e4e5520414e5c58071e4200000000000000322c083a41035741182e471b0d4c1f0c0732221f0314034d293731151633441c00020000002a1f4417144c2d542c156063336331613d1e5654182c63634705275e1822585d37095f625305234f4f3d1d263615601a385d0335095a3e1903052a0b0f1954264a020000005e303a062f2f322f3253570e032149372e550f2a4d23225a5946482d5e393914090811033e0704491716142d2e2b0a004d3a5e613f1653063147505b053806145000480e0a7115af2e40dc5e9c1890fcda18ca4b60ec5d1393d1416e9eafe537ec82000000000000000000000000000000000000000000000000000000000000000002280f130c2d092c3c2d5f012c611f4e5c624a094121393316185f4a1a461e105f2a522b24153f0a2c441a4f013f5c03032a46561e231e114246391e26470c2b02000000431c074c29614b622b23044502565a0e1b3c081c370557075f1f1e59594609474b01335b2754283211422a5458325a06090b502e524a352e2d425c18043b38572b0200000045630b03625210214c3a5a4d2731380f37313b50100b1d0a373754471e1e593234013461623256024d5a130354302e463344161d17103c4e272e31114d311e0402
```
1. Fragment size (u32): `00000572`
2. `00`
3. Fragment id tag (u8): `0b` == `11` (it is equal to VoteCast tag)
4. Vote plan id (32 byte hash): `93c52748d7f75ffb0267846af7b3ca5228d7a210908be3fc963f227ab7f039f8`
5. Proposal index (u8): `1f`
6. Payload type tag (u8): `02`
7. Encrypted vote: `03|bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|4c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1a|bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|4c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1a|bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|f0d6dd25bab8405561d5bb817d9b20236bd5bb88c21b73afc09818ce5c5e4833`
    - size (u8): `03` 
    - ciphertext (group element (32 byte), group element (32 byte)): `bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|4c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1a|bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|4c03526a8ba6ca343ed1fcf61bb3b82821d0f2361f541727a3e85f0d1a55aa1a|bc02c5ed4c96454223b073ec02f8a941e7dd58abcdefa547b497969859026305|f0d6dd25bab8405561d5bb817d9b20236bd5bb88c21b73afc09818ce5c5e4833`
8. Proof: `02|864143d55bdc29e4f86db8fc1fdee2e1e9cd7062e5e2fb5ca52f581fdb7cd75b|5e7836744ecbac60802ca39e3d92ebdac06479ec27b410140488bcb73ff76f3b|caa135fe2f52ed3fcee49e7e44feb9948ab2ee242c052fb24fe03e74f5d31f4e|da0263ac2b0fddb4b56a7210e892f306b2bec41a0e9f6d713980753418ca1a6f|5e7d4ad2a52627d4f72d86757a052794d07798178b5e05068993222bf167af74|d40b167e06b063ec6f7fdd7a907b27a030197c4d7e5b53e9828be208ba94e64e|f286a900fe795a3219e1dc23a4b033ca300cbf350d48d655bcb6ce5de7bf007a|2a0ee3305468ba753fca17f700d102e665115acfc0653c8636e06aee4688ed6c|a2db9928191c9e6c930f9a2ebda82c58eb7481dee601905713a593f33ce23f4e|0ca87a1b487a36544d10c6bffeb83c3e897f968cba066471dd0cfe540dfeee2a|9ef2f8441c4cce519a90820147be22efbe06d9b20d72ed3fe3ffe4198dac0204|59a4e56d0f4beb687661c9c38777c4034c6dd0d5efbaa713df7971fbf23b1e05|32c5813d8fdff7e4dd3ea3da8755ea457a0ca5681c555dccd915db9530c0f20a|eba371fc09b1524dbef709694f1ef87d168392dfb9a17b23e05980436e2c4e0b|5aa4653e54062da822e6849d45ffa8ad7b8d9e258124ecd5adb99c4986cd3907|e7a24b5b93fe207cfe8d7ebf927f3932b314c649ec18b19958c9b5d63c442609|21b79b3d5bd79021901e2a7f4de8ad42fd677b3f033cf1b0f2151cc221703105`
    - size (u8): `02`
    - announcements (group element (32 byte), group element (32 byte), group element (32 byte)): `864143d55bdc29e4f86db8fc1fdee2e1e9cd7062e5e2fb5ca52f581fdb7cd75b|5e7836744ecbac60802ca39e3d92ebdac06479ec27b410140488bcb73ff76f3b|caa135fe2f52ed3fcee49e7e44feb9948ab2ee242c052fb24fe03e74f5d31f4e|da0263ac2b0fddb4b56a7210e892f306b2bec41a0e9f6d713980753418ca1a6f|5e7d4ad2a52627d4f72d86757a052794d07798178b5e05068993222bf167af74|d40b167e06b063ec6f7fdd7a907b27a030197c4d7e5b53e9828be208ba94e64e`
    - ciphertext (group element (32 byte), group element (32 byte)): `f286a900fe795a3219e1dc23a4b033ca300cbf350d48d655bcb6ce5de7bf007a|2a0ee3305468ba753fca17f700d102e665115acfc0653c8636e06aee4688ed6c|a2db9928191c9e6c930f9a2ebda82c58eb7481dee601905713a593f33ce23f4e|0ca87a1b487a36544d10c6bffeb83c3e897f968cba066471dd0cfe540dfeee2a`
    - response randomness (scalar (32 byte), scalar (32 byte), scalar (32 byte)): `9ef2f8441c4cce519a90820147be22efbe06d9b20d72ed3fe3ffe4198dac0204|59a4e56d0f4beb687661c9c38777c4034c6dd0d5efbaa713df7971fbf23b1e05|32c5813d8fdff7e4dd3ea3da8755ea457a0ca5681c555dccd915db9530c0f20a|eba371fc09b1524dbef709694f1ef87d168392dfb9a17b23e05980436e2c4e0b|5aa4653e54062da822e6849d45ffa8ad7b8d9e258124ecd5adb99c4986cd3907|e7a24b5b93fe207cfe8d7ebf927f3932b314c649ec18b19958c9b5d63c442609`
    - scalar (32 byte): `21b79b3d5bd79021901e2a7f4de8ad42fd677b3f033cf1b0f2151cc221703105`
9. IOW stand for Inputs-Outputs-Witnesses: `000000010000000005001d000000000000003433503f28035b5b470f3c38571e1748474e191c362c583057472941066031093e1e000000000000003d23541c2c601e5e281533631c284a2c093437044a0c07545e5505341b1e4722330700000000000000284f51171a2b5b0a2e4632452d125b3451170a004d182235390f484c2d1f2e22354300000000000000122a0e5d3608025f4b1b10281e2f2e252d2b56251734541e4e5520414e5c58071e4200000000000000322c083a41035741182e471b0d4c1f0c0732221f0314034d293731151633441c00020000002a1f4417144c2d542c156063336331613d1e5654182c63634705275e1822585d37095f625305234f4f3d1d263615601a385d0335095a3e1903052a0b0f1954264a020000005e303a062f2f322f3253570e032149372e550f2a4d23225a5946482d5e393914090811033e0704491716142d2e2b0a004d3a5e613f1653063147505b053806145000480e0a7115af2e40dc5e9c1890fcda18ca4b60ec5d1393d1416e9eafe537ec82000000000000000000000000000000000000000000000000000000000000000002280f130c2d092c3c2d5f012c611f4e5c624a094121393316185f4a1a461e105f2a522b24153f0a2c441a4f013f5c03032a46561e231e114246391e26470c2b02000000431c074c29614b622b23044502565a0e1b3c081c370557075f1f1e59594609474b01335b2754283211422a5458325a06090b502e524a352e2d425c18043b38572b0200000045630b03625210214c3a5a4d2731380f37313b50100b1d0a373754471e1e593234013461623256024d5a130354302e463344161d17103c4e272e31114d311e0402`
    - block date (epoch (u32), slot (u32)): `00000001|00000000`
    - number of inputs and witnesses (u8): `05`
    - number of outputs (u8): `00`
    - Inputs
    1. 
        - index or accout (u8): `1d` (index)
        - value (u64): `0000000000000034`
        - input pointer (32 byte): `33503f28035b5b470f3c38571e1748474e191c362c583057472941066031093e`
    2. 
        - index or accout (u8): `1e` (index)
        - value (u64): `000000000000003d`
        - input pointer (32 byte): `23541c2c601e5e281533631c284a2c093437044a0c07545e5505341b1e472233`
    3. 
        - index or accout (u8): `07` (index)
        - value (u64): `0000000000000028`
        - input pointer (32 byte): `4f51171a2b5b0a2e4632452d125b3451170a004d182235390f484c2d1f2e2235`
    4. 
        - index or accout (u8): `43` (index)
        - value (u64): `0000000000000012`
            - input pointer (32 byte): `2a0e5d3608025f4b1b10281e2f2e252d2b56251734541e4e5520414e5c58071e`
    5. 
        - index or accout (u8): `42` (index)
        - value (u64): `0000000000000032`
        - input pointer (32 byte): `2c083a41035741182e471b0d4c1f0c0732221f0314034d293731151633441c00`
    - Witnesses
    1. 
        - witness type tag (u8): `02`
        - nonce (u32): `0000002a`
        - legacy signature (64 byte): `1f4417144c2d542c156063336331613d1e5654182c63634705275e1822585d37095f625305234f4f3d1d263615601a385d0335095a3e1903052a0b0f1954264a`
    2. 
        - witness type tag (u8): `02`
        - nonce (u32): `0000005e`
        - signature (64 byte): `303a062f2f322f3253570e032149372e550f2a4d23225a5946482d5e393914090811033e0704491716142d2e2b0a004d3a5e613f1653063147505b0538061450`
    3. 
        - witness type tag (u8): `00`
        - legacy public key (64 byte): `480e0a7115af2e40dc5e9c1890fcda18ca4b60ec5d1393d1416e9eafe537ec820000000000000000000000000000000000000000000000000000000000000000`
        - legacy signature (64 byte): `02280f130c2d092c3c2d5f012c611f4e5c624a094121393316185f4a1a461e105f2a522b24153f0a2c441a4f013f5c03032a46561e231e114246391e26470c2b`
    4. 
        - witness type tag (u8): `02`
        - nonce (u32): `00000043`
        - signature (64 byte): `303a062f2f322f3253570e032149372e550f2a4d23225a5946482d5e393914090811033e0704491716142d2e2b0a004d3a5e613f1653063147505b0538061450`
    5. 
        - witness type tag (u8): `02`
        - nonce (u32): `00000045`
        - signature (64 byte): `630b03625210214c3a5a4d2731380f37313b50100b1d0a373754471e1e593234013461623256024d5a130354302e463344161d17103c4e272e31114d311e0402`

## Shared formats

Delegation Type has 3 different encodings:

```
No Delegation:

    00

Full delegation to 1 node:

    01 POOL-ID

Ratio delegation:

    Byte(PARTS) Byte(#POOLS) ( Byte(POOL-PARTS) POOL-ID )[#POOLS times]

    with PARTS >= 2 and #POOLS >= 2
```

Thus the encodings in hexadecimal:

```

No Delegation:

    00

Full Delegation to POOL-ID f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0

    01 f0 f0 f0 f0 f0 f0 f0  f0 f0 f0 f0 f0 f0 f0 f0
    f0 f0 f0 f0 f0 f0 f0 f0  f0 f0 f0 f0 f0 f0 f0 f0
    f0

Ratio Delegation of:
* 1/4 to POOL-ID f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0
* 3/4 to POOL-ID abababababababababababababababababababababababababababababababab

    04 02 01 f0 f0 f0 f0 f0  f0 f0 f0 f0 f0 f0 f0 f0
    f0 f0 f0 f0 f0 f0 f0 f0  f0 f0 f0 f0 f0 f0 f0 f0
    f0 f0 f0 03 ab ab ab ab  ab ab ab ab ab ab ab ab
    ab ab ab ab ab ab ab ab  ab ab ab ab ab ab ab ab
    ab ab ab ab
```