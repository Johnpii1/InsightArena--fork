import {
  Controller,
  Get,
  Post,
  Param,
  Body,
  Query,
  HttpCode,
  HttpStatus,
} from '@nestjs/common';
import {
  ApiOperation,
  ApiResponse,
  ApiBearerAuth,
  ApiTags,
} from '@nestjs/swagger';
import { MarketsService } from './markets.service';
import { Market } from './entities/market.entity';
import { CreateMarketDto } from './dto/create-market.dto';
import {
  ListMarketsDto,
  PaginatedMarketsResponse,
} from './dto/list-markets.dto';
import { CurrentUser } from '../common/decorators/current-user.decorator';
import { Public } from '../common/decorators/public.decorator';
import { User } from '../users/entities/user.entity';

@ApiTags('Markets')
@Controller('markets')
export class MarketsController {
  constructor(private readonly marketsService: MarketsService) {}

  @Post()
  @HttpCode(HttpStatus.CREATED)
  @ApiBearerAuth()
  @ApiOperation({ summary: 'Create a new prediction market' })
  @ApiResponse({ status: 201, description: 'Market created', type: Market })
  @ApiResponse({ status: 400, description: 'Validation error' })
  @ApiResponse({ status: 502, description: 'Soroban contract call failed' })
  async createMarket(
    @Body() dto: CreateMarketDto,
    @CurrentUser() user: User,
  ): Promise<Market> {
    return this.marketsService.create(dto, user);
  }

  @Get()
  @Public()
  @ApiOperation({ summary: 'List and filter markets with pagination' })
  @ApiResponse({
    status: 200,
    description: 'Paginated markets list',
  })
  async listMarkets(
    @Query() query: ListMarketsDto,
  ): Promise<PaginatedMarketsResponse> {
    return this.marketsService.findAllFiltered(query);
  }

  @Get(':id')
  @Public()
  @ApiOperation({ summary: 'Fetch market by ID or on-chain ID' })
  @ApiResponse({
    status: 200,
    description: 'Market with nested creator profile',
    type: Market,
  })
  @ApiResponse({ status: 404, description: 'Market not found' })
  async getMarketById(@Param('id') id: string): Promise<Market> {
    return this.marketsService.findByIdOrOnChainId(id);
  }
}
