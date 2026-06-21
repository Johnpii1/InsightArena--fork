import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Market } from './entities/market.entity';
import { Comment } from './entities/comment.entity';
import { MarketTemplate } from './entities/market-template.entity';
import { UserBookmark } from './entities/user-bookmark.entity';
import { Prediction } from '../predictions/entities/prediction.entity';
import { MarketsService } from './markets.service';
import { MarketsController } from './markets.controller';
import { UsersModule } from '../users/users.module';
import { AnalyticsModule } from '../analytics/analytics.module';
import { DisputesModule } from '../disputes/disputes.module';
import { WebhooksModule } from '../webhooks/webhooks.module';

@Module({
  imports: [
    TypeOrmModule.forFeature([
      Market,
      Comment,
      MarketTemplate,
      UserBookmark,
      Prediction,
    ]),
    UsersModule,
    AnalyticsModule,
    DisputesModule,
    WebhooksModule,
  ],
  controllers: [MarketsController],
  providers: [MarketsService],
  exports: [MarketsService, TypeOrmModule],
})
export class MarketsModule {}
