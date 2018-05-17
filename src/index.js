import 'dotenv/config'
import Koa from 'koa'
import Redis from 'ioredis'

import compress from 'koa-compress'
import Router from 'koa-router'
import serve from 'koa-static'

const app = new Koa()
const redis = new Redis(process.env.REDIS_DSN)

// Shrink that stuff up a bit
app.use(compress())

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

// Boot the server
app.listen(process.env.PORT || 3001)
