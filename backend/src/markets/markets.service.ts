import {
  Injectable,
  NotFoundException,
  BadGatewayException,
  Logger,
} from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Market } from './entities/market.entity';
import { CreateMarketDto } from './dto/create-market.dto';
import { UsersService } from '../users/users.service';
import { User } from '../users/entities/user.entity';
import {
  ListMarketsDto,
  MarketStatus,
  PaginatedMarketsResponse,
} from './dto/list-markets.dto';

@Injectable()
export class MarketsService {
  private readonly logger = new Logger(MarketsService.name);

  constructor(
    @InjectRepository(Market)
    private readonly marketsRepository: Repository<Market>,
    private readonly usersService: UsersService,
  ) {}

  /**
   * Create a new market: call Soroban contract, then persist to DB.
   * Rolls back the DB write if the Soroban call fails.
   */
  async create(dto: CreateMarketDto, user: User): Promise<Market> {
    // Step 1: Call Soroban contract to create market on-chain
    let onChainMarketId: string;
    try {
      // TODO: Replace with real SorobanService.createMarket() call
      onChainMarketId = `market_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      this.logger.log(
        `Soroban createMarket called for "${dto.title}" — on_chain_id: ${onChainMarketId}`,
      );
    } catch (err) {
      this.logger.error('Soroban createMarket failed', err);
      throw new BadGatewayException('Failed to create market on Soroban');
    }

    // Step 2: Persist to database
    try {
      const market = this.marketsRepository.create({
        on_chain_market_id: onChainMarketId,
        creator: user,
        title: dto.title,
        description: dto.description,
        category: dto.category,
        outcome_options: dto.outcome_options,
        end_time: new Date(dto.end_time),
        resolution_time: new Date(dto.resolution_time),
        is_public: dto.is_public,
        is_resolved: false,
        is_cancelled: false,
        total_pool_stroops: '0',
        participant_count: 0,
      });

      return await this.marketsRepository.save(market);
    } catch (err) {
      this.logger.error('Failed to save market to DB after Soroban success', err);
      throw new BadGatewayException(
        'Market created on-chain but failed to save to database',
      );
    }
  }

  /**
   * List markets with pagination, filtering, and keyword search.
   */
  async findAllFiltered(
    dto: ListMarketsDto,
  ): Promise<PaginatedMarketsResponse> {
    const page = dto.page ?? 1;
    const limit = Math.min(dto.limit ?? 20, 50);
    const skip = (page - 1) * limit;

    const qb = this.marketsRepository
      .createQueryBuilder('market')
      .leftJoinAndSelect('market.creator', 'creator');

    if (dto.category) {
      qb.andWhere('market.category = :category', { category: dto.category });
    }

    if (dto.status) {
      switch (dto.status) {
        case MarketStatus.Open:
          qb.andWhere(
            'market.is_resolved = false AND market.is_cancelled = false',
          );
          break;
        case MarketStatus.Resolved:
          qb.andWhere('market.is_resolved = true');
          break;
        case MarketStatus.Cancelled:
          qb.andWhere('market.is_cancelled = true');
          break;
      }
    }

    if (dto.is_public !== undefined) {
      qb.andWhere('market.is_public = :is_public', {
        is_public: dto.is_public,
      });
    }

    if (dto.search) {
      qb.andWhere('market.title ILIKE :search', {
        search: `%${dto.search}%`,
      });
    }

    qb.orderBy('market.created_at', 'DESC').skip(skip).take(limit);

    const [data, total] = await qb.getManyAndCount();

    return { data, total, page, limit };
  }

  async findAll(): Promise<Market[]> {
    return this.marketsRepository.find({
      relations: ['creator'],
    });
  }

  /**
   * Find a market by UUID or on_chain_market_id.
   */
  async findByIdOrOnChainId(id: string): Promise<Market> {
    const market = await this.marketsRepository.findOne({
      where: [{ id }, { on_chain_market_id: id }],
      relations: ['creator'],
    });

    if (!market) {
      throw new NotFoundException(`Market with ID "${id}" not found`);
    }

    return market;
  }
}
