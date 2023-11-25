# Fetch imported portfolio

Fetch previously imported portfolio, if not expired yet. 

**URL** : `/import/portfolio/:id`

| **Parameter** | **Type** | **Description** |
| --- | --- | --- |
| `id` | `string` | ID of the imported portfolio to fetch |

**Method** : `GET`

**Auth required** : NO

**Permissions required** : None

**Data constraints** : None

**Header constraints** : None

**Data examples** : Empty

## Success Responses

**Condition** : Portfolio with `id` exists and has not expired yet.

**Code** : `200 OK`

**Content example** :

```json
{
  "name": "My Portfolio",
  "quoteCcy": "usd",
  "fees": {
    "feeStructure": {
      "type": "variable",
      "feeRate": 0.19,
      "minFee": 2.95
    }
  },
  "assets": [
    {
      "symbol": "btc",
      "name": "Bitcoin",
      "aclass": "CRYPTO",
      "baseCcy": "btc",
      "provider": "DCAPal",
      "price": 37190.1,
      "qty": 0.1,
      "targetWeight": 100.0
    }
  ]
}
```

## Error Responses

**Condition** : Portfolio with `id` does not exists or it has already expired

**Code** : `404 NOT FOUND`

**Context example** : Empty
