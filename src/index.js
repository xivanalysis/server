import 'dotenv/config'
import axios from 'axios'
import Koa from 'koa'
import Redis from 'ioredis'

import compress from 'koa-compress'
import cors from '@koa/cors'
import mount from 'koa-mount'
import Router from 'koa-router'
import serve from 'koa-static'

const app = new Koa()
const redis = new Redis(process.env.REDIS_DSN)

// Shrink that stuff up a bit
app.use(compress())

// Loosen up CORS a bit
app.use(cors())

// Serve the main public assets
// TODO: Would be nice to be able to hit a url without the /#/
app.use(serve(process.env.PUBLIC_PATH || 'public'))

// Set up the router for the API
const router = new Router()
router.get('/api', async ctx => {
	ctx.body = await redis.get('test')
})

app.use(router.routes())
app.use(router.allowedMethods())

// FFLogs proxy
const fflogsApi = axios.create({
	baseURL: 'https://www.fflogs.com/v1/'
})

app.use(mount('/proxy/fflogs', async ctx => {
	// Default to our api key, but allow overrides
	let apiKey = process.env.FFLOGS_API_KEY
	const query = ctx.query
	if (query.api_key) {
		apiKey = query.api_key
		delete query.api_key
		ctx.query = query
	}

	// Build the URL that we'll be requesting (and using as a key!)
	const url = ctx.request.url

	// Check if we have a cached copy, return that if we do
	const key = 'proxy:fflogs:' + url
	if (await redis.exists(key)) {
		ctx.body = await redis.get(key)
		return
	}

	// No cached copy, grab it down
	let response = null
	try {
		response = await fflogsApi.get(url, {
			params: {api_key: apiKey}
		})
	} catch(e) {
		// TODO: Handle this better.
		ctx.body = JSON.stringify([e.response.status, e.response.data])
		return
	}

	// Save and respond
	const data = response.data
	redis.set(key, JSON.stringify(data))
	ctx.body = data
}))

// Boot the server
app.listen(process.env.PORT || 3001)
