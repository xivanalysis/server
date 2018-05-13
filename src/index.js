import 'dotenv/config'
import Koa from 'koa'
import Router from 'koa-router'

const app = new Koa()
const router = new Router()

router.get('/', ctx => {
	ctx.body = 'This was a triumph!'
})

app.use(router.routes())
app.use(router.allowedMethods())
app.listen(process.env.PORT || 3001)
