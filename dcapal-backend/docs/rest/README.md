# DcaPal REST API Documentation

This document provides REST API endpoints documentation for DcaPal backend. 

Endpoints are divided in *public* and *internal* families: public APIs are intended for Developers wishing to integrate their tools or apps with DcaPal, whereas *internal* APIs are meant to be called by DcaPal client only. Furthermore, endpoints are classified as *open* or *authorized*: open APIs can be freely called and do not require authentication, while authorized APIs require authentication and access to resources is constrained by visibility rules.

> ### Notes
> *When not specified, endpoints always assume `https://dcapal.com/api/` base URL*

## Public endpoints

In this section you can find DcaPal REST APIs intended for Developers, wishing to integrate their tools or apps with DcaPal.

### Open endpoints

#### Import Portfolio

Developers can integrate with DcaPal importing their portfolios via REST API and later open it in the client

- [Import portfolio](public/import/post.md): `POST /import/portfolio`
- [Fetch imported portfolio](public/import/get.md): `GET /import/portfolio/:id`

### Authorized endpoints

## Internal endpoints

### Open endpoints

TBD

### Authorized endpoints

TBD