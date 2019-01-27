import axios from 'axios'
import Koa from 'koa'
import Router from 'koa-router'

import redis from './redis'

// Set up the instance
const app = new Koa()
const router = new Router()

const XIVAPI_BASE_URL = 'https://xivapi.com'

// Set up base params to connect to xivapi
const xivapi = axios.create({
	baseURL: XIVAPI_BASE_URL,
	params: {
		key: 'ea807e43a1524c31bcdc0cbd',
	},
})

router.get('/zone-banner/:zoneId(\\d+)', async ctx => {
	const {zoneId} = ctx.params

	// Do a lookup for the banner
	const key = `xivapi:zoneBanner:${zoneId}`
	let banner = await redis.get(key)

	// If we don't have a cached path, get a fresh one from the api
	if (!banner) {
		// Spooky magic request that gets what we need
		const {data} = await xivapi.get('search', {params: {
			indexes: 'InstanceContent',
			filters: `ContentFinderCondition.TerritoryTypeTargetID=${zoneId}`,
			columns: 'Banner',
		}})

		// If there's no results, tell the user off for being a numpty
		if (data.Pagination.Results === 0) {
			// eslint-disable-next-line no-magic-numbers
			ctx.throw(400, 'invalid zone id provided')
		}

		// Grab the banner and fire off a set (don't need to wait)
		banner = data.Results[0].Banner
		redis.set(key, banner)
	}

	// 303 to the image, we're acting as a pass-through "nice" url
	ctx.status = 303
	ctx.redirect(XIVAPI_BASE_URL + banner)
})

app.use(router.routes())
app.use(router.allowedMethods())

export default app
