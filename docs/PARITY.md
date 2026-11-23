# Parity checklist

Endpoints the official client calls, listed from the route stubs of DiscordBotClient (used only as an index; no code taken). These are the endpoints that project had to stub for bot accounts, so the list covers account features (settings, billing, profile, relationships…) more than core messaging, which is tracked by the phoenix-api tests. Tick each one once Discord Phoenix implements and uses it.


## /activities
- [ ] `/activities/shelf`

## /applications
- [ ] `/applications/non-games/detectable`

## /applications-with-assets
- [ ] `/applications-with-assets`

## /app-recommendations
- [ ] `/app-recommendations`

## /auth
- [ ] `/auth/location-metadata`
- [ ] `/auth/logout`
- [ ] `/auth/sessions`

## /cdn-cgi
- [ ] `/cdn-cgi`

## /channels
- [ ] `/channels/1000000000000000000/messages`
- [ ] `/channels/{id}/application-command-index`
- [ ] `/channels/{id}/application-commands/search`
- [ ] `/channels/{id}/explicit-media`
- [x] `/channels/{id}/messages/#mid/ack`
- [ ] `/channels/{id}/messages/search`
- [ ] `/channels/{id}/polls/#pid/answers/@me`
- [ ] `/channels/{id}/post-data`
- [ ] `/channels/{id}/threads/search`

## /checkpoint
- [ ] `/checkpoint`

## /collectibles-categories
- [ ] `/collectibles-categories`
- [ ] `/collectibles-categories/v2`

## /collectibles-marketing
- [ ] `/collectibles-marketing`

## /collectibles-purchases
- [ ] `/collectibles-purchases`

## /collectibles-shop
- [ ] `/collectibles-shop`

## /content-inventory
- [ ] `/content-inventory/users/{id}/outbox`
- [ ] `/content-inventory/users/@me`

## /creator-monetization
- [ ] `/creator-monetization/marketing/nag-activate/eligibility`

## /developers
- [ ] `/developers/docs`
- [ ] `/developers/docs/intro`

## /__development
- [ ] `/__development/build_overrides`

## /discoverable-guilds
- [ ] `/discoverable-guilds`
- [ ] `/discoverable-guilds/search`

## /download
- [ ] `/download`

## /error-reporting-proxy
- [ ] `/error-reporting-proxy/web`

## /experiments
- [ ] `/experiments`

## /family-center
- [ ] `/family-center/@me`

## /games
- [ ] `/games`

## /guilds
- [ ] `/guilds/{id}/admin-server-eligibility`
- [ ] `/guilds/{id}/analytics/member-insights`
- [ ] `/guilds/{id}/application-command-index`
- [ ] `/guilds/{id}/applications`
- [ ] `/guilds/{id}/entitlements`
- [ ] `/guilds/{id}/members/@me`
- [ ] `/guilds/{id}/migrate-command-scope`
- [ ] `/guilds/{id}/onboarding-responses`
- [ ] `/guilds/{id}/powerups`
- [ ] `/guilds/{id}/premium/subscriptions`
- [ ] `/guilds/{id}/products/listings`
- [ ] `/guilds/{id}/profile`
- [ ] `/guilds/{id}/role-subscriptions/group-listings`
- [ ] `/guilds/{id}/role-subscriptions/settings`
- [ ] `/guilds/{id}/role-subscriptions/trials`
- [ ] `/guilds/{id}/top-emojis`
- [ ] `/guilds/{id}/top-games`
- [ ] `/guilds/integration-application-ids`
- [ ] `/guilds/premium/subscription-slots`

## /hypesquad
- [ ] `/hypesquad`

## /oauth2
- [ ] `/oauth2/authorize`
- [ ] `/oauth2/tokens`

## /outbound-promotions
- [ ] `/outbound-promotions`

## /partner-sdk
- [ ] `/partner-sdk/storefront-config`

## /ping
- [ ] `/ping`

## /popout
- [ ] `/popout`

## /premium-marketing
- [ ] `/premium-marketing`

## /quests
- [ ] `/quests/decision`
- [ ] `/quests/get-decisions`
- [ ] `/quests/{id}`
- [ ] `/quests/@me`
- [ ] `/quests/@me/claimed`

## /read-states
- [ ] `/read-states/ack-bulk`

## /safety-hub
- [ ] `/safety-hub/@me`

## /storefront
- [ ] `/storefront/promotions`

## /store
- [ ] `/store/published-listings/applications/{id}/subscription-plans`
- [ ] `/store/published-listings/skus`
- [ ] `/store/published-listings/skus/{id}/subscription-plans`

## /streams
- [ ] `/streams/#hash/preview`

## /teams
- [ ] `/teams`

## /users
- [ ] `/users/1056491867375673424`
- [ ] `/users/1056491867375673424/profile`
- [ ] `/users/%40me/badges`
- [ ] `/users/%40me/profile`
- [ ] `/users/{id}/application-command-index`
- [ ] `/users/{id}/application-identities`
- [ ] `/users/{id}/profile`
- [ ] `/users/@me`
- [ ] `/users/@me/activities/statistics/applications`
- [ ] `/users/@me/affinities/guilds`
- [ ] `/users/@me/affinities/users`
- [ ] `/users/@me/application-command-index`
- [ ] `/users/@me/applications/{id}/entitlements`
- [ ] `/users/@me/avatars`
- [ ] `/users/@me/billing/campaign-context`
- [ ] `/users/@me/billing/country-code`
- [ ] `/users/@me/billing/location-info`
- [ ] `/users/@me/billing/nitro-affinity`
- [ ] `/users/@me/billing/payments`
- [ ] `/users/@me/billing/payment-sources`
- [ ] `/users/@me/billing/subscriptions`
- [ ] `/users/@me/billing/subscriptions/{id}/preview`
- [ ] `/users/@me/billing/subscriptions/{id}/reward-eligibility`
- [ ] `/users/@me/collectibles-marketing`
- [ ] `/users/@me/collectibles-purchases`
- [ ] `/users/@me/connections`
- [ ] `/users/@me/consent`
- [ ] `/users/@me/custom-themes`
- [ ] `/users/@me/email-settings`
- [ ] `/users/@me/emojis`
- [ ] `/users/@me/entitlements`
- [ ] `/users/@me/entitlements/gifts`
- [ ] `/users/@me/gift-intent-dismissals`
- [ ] `/users/@me/guilds/integration-application-ids`
- [ ] `/users/@me/guilds/premium/subscriptions`
- [ ] `/users/@me/guilds/premium/subscription-slots`
- [ ] `/users/@me/harvest`
- [ ] `/users/@me/inventory`
- [ ] `/users/@me/library`
- [ ] `/users/@me/meaningfully-online`
- [ ] `/users/@me/mentions`
- [ ] `/users/@me/mfa/webauthn/credentials`
- [ ] `/users/@me/notification-center/items`
- [ ] `/users/@me/outbound-promotions/codes`
- [ ] `/users/@me/profile`
- [ ] `/users/@me/program-rewards`
- [ ] `/users/@me/referrals/eligibility`
- [ ] `/users/@me/referrals/eligible-users`
- [ ] `/users/@me/referrals/{id}/preview`
- [x] `/users/@me/relationships`
- [ ] `/users/@me/saved-messages`
- [x] `/users/@me/settings-proto`
- [ ] `/users/@me/survey`
- [ ] `/users/@me/unclaimed-games`
- [ ] `/users/@me/video-filters/assets`
- [ ] `/users/@me/virtual-currency/balance`
- [ ] `/users/@me/widgets/application-widget-application-ids`
- [ ] `/users/@me/widgets/suggested-games`
