use crate::{U64, U8};
use alloy_rlp::{Decodable, Encodable};
use bytes::Buf;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use reth_codecs::Compact;

/// For backwards compatibility purposes only 2 bits of the type are encoded in the identifier
/// parameter. In the case of a 3, the full transaction type is read from the buffer as a
/// single byte.
const COMPACT_EXTENDED_IDENTIFIER_FLAG: usize = 3;

/// Identifier for legacy transaction, however [`TxLegacy`](crate::TxLegacy) this is technically not
/// typed.
pub const LEGACY_TX_TYPE_ID: u8 = 0;

/// Identifier for [`TxEip2930`](crate::TxEip2930) transaction.
pub const EIP2930_TX_TYPE_ID: u8 = 1;

/// Identifier for [`TxEip1559`](crate::TxEip1559) transaction.
pub const EIP1559_TX_TYPE_ID: u8 = 2;

/// Identifier for [`TxEip4844`](crate::TxEip4844) transaction.
pub const EIP4844_TX_TYPE_ID: u8 = 3;

/// Identifier for [`TxEip7702`](crate::TxEip7702) transaction.
pub const EIP7702_TX_TYPE_ID: u8 = 4;

/// Identifier for [`TxDeposit`](crate::TxDeposit) transaction.
#[cfg(feature = "optimism")]
pub const DEPOSIT_TX_TYPE_ID: u8 = 126;

/// Transaction Type
///
/// Currently being used as 2-bit type when encoding it to `reth_codecs::Compact` on
/// [`crate::TransactionSignedNoHash`]. Adding more transaction types will break the codec and
/// database format.
///
/// Other required changes when adding a new type can be seen on [PR#3953](https://github.com/paradigmxyz/reth/pull/3953/files).
#[cfg_attr(any(test, feature = "reth-codec"), reth_codecs::derive_arbitrary(compact))]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, Hash,
)]
pub enum TxType {
    /// Legacy transaction pre EIP-2929
    #[default]
    Legacy = 0_isize,
    /// AccessList transaction
    Eip2930 = 1_isize,
    /// Transaction with Priority fee
    Eip1559 = 2_isize,
    /// Shard Blob Transactions - EIP-4844
    Eip4844 = 3_isize,
    /// EOA Contract Code Transactions - EIP-7702
    Eip7702 = 4_isize,
    /// Optimism Deposit transaction.
    #[cfg(feature = "optimism")]
    Deposit = 126_isize,
}

impl TxType {
    /// The max type reserved by an EIP.
    pub const MAX_RESERVED_EIP: Self = Self::Eip7702;

    /// Check if the transaction type has an access list.
    pub const fn has_access_list(&self) -> bool {
        match self {
            Self::Legacy => false,
            Self::Eip2930 | Self::Eip1559 | Self::Eip4844 | Self::Eip7702 => true,
            #[cfg(feature = "optimism")]
            Self::Deposit => false,
        }
    }
}

impl From<TxType> for u8 {
    fn from(value: TxType) -> Self {
        match value {
            TxType::Legacy => LEGACY_TX_TYPE_ID,
            TxType::Eip2930 => EIP2930_TX_TYPE_ID,
            TxType::Eip1559 => EIP1559_TX_TYPE_ID,
            TxType::Eip4844 => EIP4844_TX_TYPE_ID,
            TxType::Eip7702 => EIP7702_TX_TYPE_ID,
            #[cfg(feature = "optimism")]
            TxType::Deposit => DEPOSIT_TX_TYPE_ID,
        }
    }
}

impl From<TxType> for U8 {
    fn from(value: TxType) -> Self {
        Self::from(u8::from(value))
    }
}

impl TryFrom<u8> for TxType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        #[cfg(feature = "optimism")]
        if value == Self::Deposit {
            return Ok(Self::Deposit)
        }

        if value == Self::Legacy {
            return Ok(Self::Legacy)
        } else if value == Self::Eip2930 {
            return Ok(Self::Eip2930)
        } else if value == Self::Eip1559 {
            return Ok(Self::Eip1559)
        } else if value == Self::Eip4844 {
            return Ok(Self::Eip4844)
        } else if value == Self::Eip7702 {
            return Ok(Self::Eip7702)
        }

        Err("invalid tx type")
    }
}

impl TryFrom<u64> for TxType {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let value: u8 = value.try_into().map_err(|_| "invalid tx type")?;
        Self::try_from(value)
    }
}

impl TryFrom<U64> for TxType {
    type Error = &'static str;

    fn try_from(value: U64) -> Result<Self, Self::Error> {
        value.to::<u64>().try_into()
    }
}

#[cfg(any(test, feature = "reth-codec"))]
impl reth_codecs::Compact for TxType {
    fn to_compact<B>(self, buf: &mut B) -> usize
    where
        B: bytes::BufMut + AsMut<[u8]>,
    {
        match self {
            Self::Legacy => 0,
            Self::Eip2930 => 1,
            Self::Eip1559 => 2,
            Self::Eip4844 => {
                buf.put_u8(self as u8);
                COMPACT_EXTENDED_IDENTIFIER_FLAG
            }
            Self::Eip7702 => {
                buf.put_u8(self as u8);
                COMPACT_EXTENDED_IDENTIFIER_FLAG
            }
            #[cfg(feature = "optimism")]
            Self::Deposit => {
                buf.put_u8(self as u8);
                COMPACT_EXTENDED_IDENTIFIER_FLAG
            }
        }
    }

    // For backwards compatibility purposes only 2 bits of the type are encoded in the identifier
    // parameter. In the case of a 3, the full transaction type is read from the buffer as a
    // single byte.
    fn from_compact(mut buf: &[u8], identifier: usize) -> (Self, &[u8]) {
        (
            match identifier {
                0 => Self::Legacy,
                1 => Self::Eip2930,
                2 => Self::Eip1559,
                COMPACT_EXTENDED_IDENTIFIER_FLAG => {
                    let extended_identifier = buf.get_u8();
                    match extended_identifier {
                        EIP4844_TX_TYPE_ID => Self::Eip4844,
                        EIP7702_TX_TYPE_ID => Self::Eip7702,
                        #[cfg(feature = "optimism")]
                        DEPOSIT_TX_TYPE_ID => Self::Deposit,
                        _ => panic!("Unsupported TxType identifier: {extended_identifier}"),
                    }
                }
                _ => panic!("Unknown identifier for TxType: {identifier}"),
            },
            buf,
        )
    }
}

impl PartialEq<u8> for TxType {
    fn eq(&self, other: &u8) -> bool {
        *self as u8 == *other
    }
}

impl PartialEq<TxType> for u8 {
    fn eq(&self, other: &TxType) -> bool {
        *self == *other as Self
    }
}

impl Encodable for TxType {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        (*self as u8).encode(out);
    }

    fn length(&self) -> usize {
        1
    }
}

impl Decodable for TxType {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let ty = u8::decode(buf)?;

        Self::try_from(ty).map_err(alloy_rlp::Error::Custom)
    }
}

#[cfg(test)]
mod tests {
    use crate::hex;
    use rand::Rng;
    use reth_codecs::Compact;

    use super::*;

    #[test]
    fn test_u64_to_tx_type() {
        // Test for Legacy transaction
        assert_eq!(TxType::try_from(U64::from(0)).unwrap(), TxType::Legacy);

        // Test for EIP2930 transaction
        assert_eq!(TxType::try_from(U64::from(1)).unwrap(), TxType::Eip2930);

        // Test for EIP1559 transaction
        assert_eq!(TxType::try_from(U64::from(2)).unwrap(), TxType::Eip1559);

        // Test for EIP4844 transaction
        assert_eq!(TxType::try_from(U64::from(3)).unwrap(), TxType::Eip4844);

        // Test for EIP7702 transaction
        assert_eq!(TxType::try_from(U64::from(4)).unwrap(), TxType::Eip7702);

        // Test for Deposit transaction
        #[cfg(feature = "optimism")]
        assert_eq!(TxType::try_from(U64::from(126)).unwrap(), TxType::Deposit);

        // For transactions with unsupported values
        assert!(TxType::try_from(U64::from(5)).is_err());
    }

    #[test]
    fn test_txtype_to_compat() {
        let cases = vec![
            (TxType::Legacy, 0, vec![]),
            (TxType::Eip2930, 1, vec![]),
            (TxType::Eip1559, 2, vec![]),
            (TxType::Eip4844, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![EIP4844_TX_TYPE_ID]),
            (TxType::Eip7702, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![EIP7702_TX_TYPE_ID]),
            #[cfg(feature = "optimism")]
            (TxType::Deposit, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![DEPOSIT_TX_TYPE_ID]),
        ];

        for (tx_type, expected_identifier, expected_buf) in cases {
            let mut buf = vec![];
            let identifier = tx_type.to_compact(&mut buf);
            assert_eq!(
                identifier, expected_identifier,
                "Unexpected identifier for TxType {tx_type:?}",
            );
            assert_eq!(buf, expected_buf, "Unexpected buffer for TxType {tx_type:?}");
        }
    }

    #[test]
    fn test_txtype_from_compact() {
        let cases = vec![
            (TxType::Legacy, 0, vec![]),
            (TxType::Eip2930, 1, vec![]),
            (TxType::Eip1559, 2, vec![]),
            (TxType::Eip4844, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![EIP4844_TX_TYPE_ID]),
            (TxType::Eip7702, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![EIP7702_TX_TYPE_ID]),
            #[cfg(feature = "optimism")]
            (TxType::Deposit, COMPACT_EXTENDED_IDENTIFIER_FLAG, vec![DEPOSIT_TX_TYPE_ID]),
        ];

        for (expected_type, identifier, buf) in cases {
            let (actual_type, remaining_buf) = TxType::from_compact(&buf, identifier);
            assert_eq!(actual_type, expected_type, "Unexpected TxType for identifier {identifier}",);
            assert!(
                remaining_buf.is_empty(),
                "Buffer not fully consumed for identifier {identifier}",
            );
        }
    }

    #[test]
    fn decode_tx_type() {
        // Test for Legacy transaction
        let tx_type = TxType::decode(&mut &hex!("80")[..]).unwrap();
        assert_eq!(tx_type, TxType::Legacy);

        // Test for EIP2930 transaction
        let tx_type = TxType::decode(&mut &[1u8][..]).unwrap();
        assert_eq!(tx_type, TxType::Eip2930);

        // Test for EIP1559 transaction
        let tx_type = TxType::decode(&mut &[2u8][..]).unwrap();
        assert_eq!(tx_type, TxType::Eip1559);

        // Test for EIP4844 transaction
        let tx_type = TxType::decode(&mut &[3u8][..]).unwrap();
        assert_eq!(tx_type, TxType::Eip4844);

        // Test for EIP7702 transaction
        let tx_type = TxType::decode(&mut &[4u8][..]).unwrap();
        assert_eq!(tx_type, TxType::Eip7702);

        // Test random byte not in range
        let buf = [rand::thread_rng().gen_range(5..=u8::MAX)];
        assert!(TxType::decode(&mut &buf[..]).is_err());

        // Test for Deposit transaction
        #[cfg(feature = "optimism")]
        {
            let buf = [126u8];
            let tx_type = TxType::decode(&mut &buf[..]).unwrap();
            assert_eq!(tx_type, TxType::Deposit);
        }
    }
}
