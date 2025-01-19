# Sync portfolios

Sync user's portfolios with DcaPal

**URL** : `/sync/portfolios`

**Method** : `POST`

**Auth required** : YES

**Permissions required** : SYNC_PORTFOLIO

**Data constraints**

Request body must be a JSON payload matching the [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema. The ID should be provided, if not the request will be rejected.

**Header constraints** : None

**Data examples**

```json
{
  "portfolios": [
    {
      "id": "f2479b20-a873-48fd-84c3-12fd979afebd",
      "name": "My Portfolio",
      "currency": "usd",
      "fees": {
        "feeStructure": {
          "type": "variable",
          "feeRate": 0.19,
          "minFee": 2.95
        },
        "maxFeeImpact": 0.5
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
      ],
      "lastUpdatedAt": "2023-11-24 18:30:01 UTC"
    }
  ],
  "deletedPortfolios": [
    "6585fe4c-912b-4597-8c52-7970ead6e1d1"
  ]
}
```

## Success Responses

**Condition** : Data provided is successfully validated against [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema.

**Code** : `201 CREATED`

**Content example** :Response contains just updated (if the server has a greater lastUpdated) or missing portfolios for the user.

```json
{
  "portfolios": [
    {
      "id": "5035c98b63b4451380f08c4978166bec",
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
      ],
      "lastUpdatedAt": "2023-11-24 18:35:01 UTC"
    }
  ]
}
```

## Error Responses

**Condition** : Data does not meet [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema constraints.

**Code** : `400 BAD REQUEST`

**Context example**

```
Bad Request: Input portfolio does not match portfolio schema requirements
```
