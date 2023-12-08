# Import portfolio

Temporarely upload DcaPal portfolio definition to later import it in DcaPal client

**URL** : `/import/portfolio`

**Method** : `POST`

**Auth required** : NO

**Permissions required** : None

**Data constraints**

Request body must be a JSON payload matching the [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema.

**Header constraints** : None

**Data examples**

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

## Success Responses

**Condition** : Data provided is successfully validated against [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema.

**Code** : `201 CREATED`

**Content example** : Response contains an `id` of the uploaded portfolio and an expiration time. Expired resources get evicted and needs to be imported again with this endpoint.

```json
{
    "id": "5035c98b63b4451380f08c4978166bec",
    "expires_at": "2023-11-24 18:30:01 UTC"
}
```

## Error Responses

**Condition** : Data does not meet [`portfolio`](../../../schema/portfolio/v1/schema.json) JSON schema constraints.

**Code** : `400 BAD REQUEST`

**Context example**

```
Bad Request: Input portfolio does not match portfolio schema requirements
```

## Notes

- You can get further examples of valid portfolio formats by **exporting** a JSON portfolio built in DcaPal app.