-- osm2pgsql flex configuration generated from imposm3 mapping.yaml
-- Tables keep the existing schema used by the renderer under rust/crates/core

local projection = 3857;

---@param t table
local function shallow_copy(t)
    local t2 = {}
    for k, v in pairs(t) do
        t2[k] = v
    end
    return t2
end

---@param list table<number, string>
---@return table<string, true>
local function set(list)
    local t = {}
    for _, v in ipairs(list) do
        t[v] = true
    end
    return t
end

-- Normalize common OSM boolean values.
-- Returns: true | false | nil (or default if provided)
local function osm_bool(v, default)
    if v == nil then
        return default
    end

    v = tostring(v):lower()

    if v == "yes" or v == "true" or v == "1" or v == "on" then
        return true
    end

    if v == "no" or v == "false" or v == "0" or v == "off" then
        return false
    end

    return default
end

local function enum_from_tags(tags, key, allowed)
    local v = tags[key]
    if not v then
        return nil
    end

    for _, value in ipairs(allowed) do
        if v == value then
            return v
        end
    end

    return nil
end

---@param tags table<string, unknown>
---@param keys string[]
local function take_tags(tags, keys)
    local result = {}

    for _, k in ipairs(keys) do
        local v = tags[k]
        if v and v ~= "" then
            result[k] = v
        end
    end

    return result
end

---@param tags table<string, unknown>
local function z_order_for_way(tags)
    local layer = tonumber(tags.layer) or 0

    local base = 0

    local highway = tags.highway
    local railway = tags.railway
    local waterway = tags.waterway

    if railway then
        base = 5
    end

    if waterway then
        base = 3
    end

    if highway then
        local highway_rank = {
            motorway = 9,
            motorway_link = 8,
            trunk = 8,
            trunk_link = 7,
            primary = 7,
            primary_link = 6,
            secondary = 6,
            secondary_link = 5,
            tertiary = 5,
            tertiary_link = 4,
            unclassified = 4,
            residential = 4,
            living_street = 4,
            road = 4,
            service = 3,
            track = 3,
            path = 2,
            cycleway = 2,
            footway = 2,
            bridleway = 2,
            pedestrian = 2,
            steps = 1,
        }

        base = highway_rank[highway] or base
    end

    return base * 10 + layer * 10
end

local tables = {}

-- Shared column definitions reused by generalized tables


---@type OsmColumnDef[]
local landusage_columns = {
    { column = "geometry", type = "geometry", projection = projection, not_null = true },
    { column = "name",     type = "text" },
    { column = "type",     type = "text" },
    { column = "area",     type = "real",     not_null = true },
    { column = "tags",     type = "hstore",   not_null = true },
}

---@type OsmColumnDef[]
local waterway_columns = {
    { column = "geometry",     type = "linestring", projection = projection, not_null = true },
    { column = "name",         type = "text" },
    { column = "intermittent", type = "bool" },
    { column = "seasonal",     type = "bool" },
    { column = "tunnel",       type = "bool" },
    { column = "type",         type = "text" },
}

---@type OsmColumnDef[]
local road_columns = {
    { column = "geometry",         type = "linestring", projection = projection, not_null = true },
    { column = "type",             type = "text" },
    { column = "name",             type = "text" },
    { column = "tunnel",           type = "bool" },
    { column = "embankment",       type = "bool" },
    { column = "bridge",           type = "bool" },
    { column = "oneway",           type = "direction" },
    { column = "cutting",          type = "text" },
    { column = "ref",              type = "text" },
    { column = "z_order",          type = "int" },
    { column = "access",           type = "text" },
    { column = "bicycle",          type = "text" },
    { column = "foot",             type = "text" },
    { column = "vehicle",          type = "text" },
    { column = "service",          type = "text" },
    { column = "tracktype",        type = "text" },
    { column = "class",            type = "text" },
    { column = "trail_visibility", type = "int" },
    { column = "sac_scale",        type = "text" },
    { column = "fixme",            type = "text" },
}

---@type OsmColumnDef[]
local waterarea_columns = {
    { column = "geometry",     type = "geometry", projection = projection, not_null = true },
    { column = "name",         type = "text" },
    { column = "type",         type = "text" },
    { column = "area",         type = "real",     not_null = true },
    { column = "intermittent", type = "bool" },
    { column = "seasonal",     type = "bool" },
    { column = "water",        type = "text" },
}

local node_pk = { type = "node", id_column = "osm_id", create_index = "primary_key" }
local way_pk = { type = "way", id_column = "osm_id", create_index = "primary_key" }
local relation_pk = { type = "relation", id_column = "osm_id", create_index = "primary_key" }
local area_pk = { type = "area", id_column = "osm_id", create_index = "primary_key" }

tables.routes = osm2pgsql.define_table({
    name = "osm_routes",
    cluster = "no",
    ids = relation_pk,
    columns = {
        { column = "name",        type = "text" },
        { column = "ref",         type = "text" },
        { column = "colour",      type = "text" },
        { column = "state",       type = "text" },
        { column = "osmc:symbol", type = "text" },
        { column = "network",     type = "text" },
        { column = "type",        type = "text" },
    }
})

-- local route_member_columns = {
--     { column = "member",   type = "bigint" },
--     { column = "geometry", type = "geometry", projection = projection },
--     { column = "role",     type = "text" },
--     { column = "type",     type = "int" },
-- }

-- tables.route_members = osm2pgsql.define_table({
--     name = "osm_route_members",
--     cluster = "no",
--     ids = { type = "relation", id_column = "osm_id" },
--     indexes = { { column = "osm_id", method = "hash" } },
--     columns = route_member_columns,
-- })

-- tables.route_members_gen1 = osm2pgsql.define_table({
--     name = "osm_route_members_gen1",
--     cluster = "no",
--     ids = { type = "relation", id_column = "osm_id" },
--     indexes = { { column = "osm_id", method = "hash" } },
--     columns = route_member_columns,
-- })

-- tables.route_members_gen0 = osm2pgsql.define_table({
--     name = "osm_route_members_gen0",
--     cluster = "no",
--     ids = { type = "relation", id_column = "osm_id" },
--     indexes = { { column = "osm_id", method = "hash" } },
--     columns = route_member_columns,
-- })

tables.landusages = osm2pgsql.define_table({
    name = "osm_landusages",
    cluster = "no",
    ids = area_pk,
    columns = landusage_columns
})

tables.landusages_gen1 = osm2pgsql.define_table({
    name = "osm_landusages_gen1",
    cluster = "no",
    ids = { type = "tile" },
    columns = landusage_columns
})

tables.landusages_gen0 = osm2pgsql.define_table({
    name = "osm_landusages_gen0",
    cluster = "no",
    ids = { type = "tile" },
    columns = landusage_columns
})

tables.buildings = osm2pgsql.define_table({
    name = "osm_buildings",
    cluster = "no",
    ids = area_pk,
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
    },
})

tables.shops = osm2pgsql.define_table({
    name = "osm_shops",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
    },
})

tables.places = osm2pgsql.define_table({
    name = "osm_places",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry",   type = "point", projection = projection, not_null = true },
        { column = "name",       type = "text",  not_null = true },
        { column = "type",       type = "text",  not_null = true },
        { column = "z_order",    type = "int" },
        { column = "population", type = "int" },
    },
})

tables.aeroways = osm2pgsql.define_table({
    name = "osm_aeroways",
    cluster = "no",
    ids = way_pk,
    columns = {
        { column = "geometry", type = "linestring", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text",       not_null = true },
    },
})

tables.waterways = osm2pgsql.define_table({
    name = "osm_waterways",
    cluster = "no",
    ids = way_pk,
    columns = waterway_columns
})


tables.waterways_gen1 = osm2pgsql.define_table({
    name = "osm_waterways_gen1",
    cluster = "no",
    ids = way_pk,
    columns =
        waterway_columns
})


tables.waterways_gen0 = osm2pgsql.define_table({
    name = "osm_waterways_gen0", cluster = "no", ids = way_pk, columns = waterway_columns })


tables.barrierways = osm2pgsql.define_table({
    name = "osm_barrierways",
    cluster = "no",
    ids = way_pk,
    columns = {
        { column = "geometry", type = "linestring", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text",       not_null = true },
    },
})

tables.barrierpoints = osm2pgsql.define_table({
    name = "osm_barrierpoints",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text",  not_null = true },
        { column = "access",   type = "text" },
    },
})

tables.feature_lines = osm2pgsql.define_table({
    name = "osm_feature_lines",
    cluster = "no",
    ids = way_pk,
    columns = {
        { column = "geometry", type = "linestring", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text",       not_null = true },
        { column = "fixme",    type = "text" },
    },
})

tables.pipelines = osm2pgsql.define_table({
    name = "osm_pipelines",
    cluster = "no",
    ids = way_pk,
    columns = {
        { column = "geometry",  type = "linestring", projection = projection, not_null = true },
        { column = "name",      type = "text" },
        { column = "location",  type = "text" },
        { column = "substance", type = "text" },
    },
})

tables.protected_areas = osm2pgsql.define_table({
    name = "osm_protected_areas",
    cluster = "no",
    ids = area_pk,
    columns = {
        { column = "geometry",      type = "geometry", projection = projection, not_null = true },
        { column = "name",          type = "text" },
        { column = "type",          type = "text" },
        { column = "protect_class", type = "text" },
        { column = "area",          type = "real",     not_null = true },
    },
})

tables.fords = osm2pgsql.define_table({
    name = "osm_fords",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "type",     type = "text" },
    },
})

tables.features = osm2pgsql.define_table({
    name = "osm_features",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry", type = "point",  projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text",   not_null = true },
        { column = "tags",     type = "hstore", not_null = true },
    },
})

tables.towers = osm2pgsql.define_table({
    name = "osm_towers",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "class",    type = "text" },
        { column = "type",     type = "text" },
        { column = "ele",      type = "text" },
    },
})

tables.feature_polys = osm2pgsql.define_table({
    name = "osm_feature_polys",
    cluster = "no",
    ids = area_pk,
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
        { column = "tags",     type = "hstore",   not_null = true },
    },
})

tables.pois = osm2pgsql.define_table({
    name = "osm_pois",
    cluster = "no",
    ids = { type = "any", type_column = "osm_type", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
        { column = "tags",     type = "jsonb", not_null = true },
    }
})

tables.springs = osm2pgsql.define_table({
    name = "osm_springs",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry",             type = "point", projection = projection, not_null = true },
        { column = "name",                 type = "text" },
        { column = "type",                 type = "text" },
        { column = "ele",                  type = "text" },
        { column = "refitted",             type = "bool" },
        { column = "seasonal",             type = "bool" },
        { column = "intermittent",         type = "bool" },
        { column = "drinking_water",       type = "text" },
        { column = "water_characteristic", type = "text" },
    },
})

tables.building_points = osm2pgsql.define_table({
    name = "osm_building_points",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry", type = "point",  projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
        { column = "tags",     type = "hstore", not_null = true },
    },
})

tables.sports = osm2pgsql.define_table({
    name = "osm_sports",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
        { column = "tags",     type = "hstore",   not_null = true },
    },
})

tables.power_generators = osm2pgsql.define_table({
    name = "osm_power_generators",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "source",   type = "text" },
        { column = "method",   type = "text" },
    },
})

tables.ruins = osm2pgsql.define_table({
    name = "osm_ruins",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "type",     type = "text" },
    },
})

tables.place_of_worships = osm2pgsql.define_table({
    name = "osm_place_of_worships",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry", type = "geometry", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "building", type = "text" },
        { column = "religion", type = "text" },
    },
})

tables.infopoints = osm2pgsql.define_table({
    name = "osm_infopoints",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "name",     type = "text" },
        { column = "ele",      type = "text" },
        { column = "foot",     type = "bool" },
        { column = "bicycle",  type = "bool" },
        { column = "ski",      type = "bool" },
        { column = "horse",    type = "bool" },
        { column = "type",     type = "text" },
    },
})

tables.aerialways = osm2pgsql.define_table({
    name = "osm_aerialways",
    cluster = "no",
    ids = way_pk,
    columns = {
        { column = "geometry", type = "linestring", projection = projection, not_null = true },
        { column = "type",     type = "text" },
        { column = "name",     type = "text" },
        { column = "ref",      type = "text" },
    },
})

tables.roads = osm2pgsql.define_table({ name = "osm_roads", cluster = "no", ids = way_pk, columns = road_columns })


tables.roads_gen1 = osm2pgsql.define_table({
    name = "osm_roads_gen1",
    cluster = "no",
    ids = way_pk,
    columns = road_columns
})


tables.roads_gen0 = osm2pgsql.define_table({
    name = "osm_roads_gen0",
    cluster = "no",
    ids = way_pk,
    columns = road_columns
})


tables.housenumbers = osm2pgsql.define_table({
    name = "osm_housenumbers",
    cluster = "no",
    ids = { type = "any", id_column = "osm_id" },
    columns = {
        { column = "geometry",    type = "point", projection = projection, not_null = true },
        { column = "housenumber", type = "text",  not_null = true },
        { column = "name",        type = "text" },
        { column = "type",        type = "text" },
    },
})

tables.waterareas = osm2pgsql.define_table({
    name = "osm_waterareas",
    cluster = "no",
    ids = area_pk,
    columns = waterarea_columns
})


tables.waterareas_gen1 = osm2pgsql.define_table({
    name = "osm_waterareas_gen1",
    cluster = "no",
    ids = area_pk,
    columns = waterarea_columns
})

tables.waterareas_gen0 = osm2pgsql.define_table({
    name = "osm_waterareas_gen0",
    cluster = "no",
    ids = area_pk,
    columns = waterarea_columns
})

tables.fixmes = osm2pgsql.define_table({
    name = "osm_fixmes",
    cluster = "no",
    ids = node_pk,
    columns = {
        { column = "geometry", type = "point", projection = projection, not_null = true },
        { column = "type",     type = "text" },
    },
})

---@param base_row table<string, unknown>
---@param geom OsmGeometry
---@param area number
local function insert_generalized_waterarea(base_row, geom, area)
    if area > 50000 then
        local g1 = geom:simplify(50)
        base_row.geometry = g1
        tables.waterareas_gen1:insert(base_row)
    end

    if area > 500000 then
        local g0 = geom:simplify(200)
        base_row.geometry = g0
        tables.waterareas_gen0:insert(base_row)
    end
end

---@param base_row table<string, unknown>
---@param geom OsmGeometry
local function insert_generalized_waterway(base_row, geom)
    local g1 = geom:simplify(50)
    base_row.geometry = g1
    tables.waterways_gen1:insert(base_row)

    local g0 = geom:simplify(200)
    base_row.geometry = g0
    tables.waterways_gen0:insert(base_row)
end

local function insert_generalized_route_member(base_row, geom)
    base_row.geometry = geom:simplify(50)
    tables.route_members_gen1:insert(base_row)

    base_row.geometry = geom:simplify(200)
    tables.route_members_gen0:insert(base_row)
end

---@param geom OsmGeometry
local function to_surface_point(geom)
    local type = geom:geometry_type();

    if type == 'POINT' or type == 'NULL' then
        return geom
    end

    local g = geom:pole_of_inaccessibility({ stretch = 1.0 })

    type = geom:geometry_type();

    if type == 'POINT' or type == 'NULL' then
        return geom
    end

    return geom:centroid();
end

local landuse_values = {
    man_made = set({
        "clearcut",
        "bunker_silo",
        "silo",
        "storage_tank",
        "wastewater_plant",
        "bridge",
    }),
    amenity = set({
        "university",
        "school",
        "college",
        "library",
        "parking",
        "hospital",
        "grave_yard",
    }),
    leisure = set({
        "park",
        "garden",
        "playground",
        "golf_course",
        "sports_centre",
        "pitch",
        "stadium",
        "dog_park",
        "track",
    }),
    tourism = set({ "zoo" }),
    natural = set({
        "bare_rock",
        "beach",
        "blockfield",
        "fell",
        "glacier",
        "grassland",
        "heath",
        "moor",
        "sand",
        "scree",
        "scrub",
        "shingle",
        "wetland",
        "wood",
    }),
    landuse = set({
        "park",
        "forest",
        "residential",
        "retail",
        "commercial",
        "industrial",
        "railway",
        "cemetery",
        "grass",
        "farmyard",
        "farm",
        "farmland",
        "orchard",
        "vineyard",
        "meadow",
        "village_green",
        "recreation_ground",
        "allotments",
        "quarry",
        "landfill",
        "brownfield",
        "greenfield",
        "depot",
        "garages",
        "military",
        "plant_nursery",
    }),
    highway = set({ "pedestrian", "footway" }),
    waterway = set({ "weir", "dam" }),
}

local feature_line_values = {
    natural = set({ "cliff", "valley", "tree_row", "ridge", "gully", "earth_bank" }),
    power = set({ "line", "minor_line" }),
    man_made = set({ "cutline", "embankment", "dyke" }),
    millitary = set({ "trench" }),
    barrier = set({ "ditch" }),
    waterway = set({ "dam", "weir" }),
}

local feature_point_values = {
    aerialway = set({ "pylon" }),
    aeroway = set({ "aerodrome" }),
    amenity = set({
        "atm",
        "bank",
        "bar",
        "bbq",
        "bench",
        "bicycle_parking",
        "bicycle_rental",
        "biergarten",
        "bus_station",
        "cafe",
        "car_wash",
        "cinema",
        "clinic",
        "college",
        "community_centre",
        "dentist",
        "doctors",
        "drinking_water",
        "fast_food",
        "feeding_place",
        "fire_station",
        "fountain",
        "fuel",
        "game_feeding",
        "hospital",
        "hunting_stand",
        "ice_cream",
        "kindergarten",
        "library",
        "monastery",
        "parking",
        "pharmacy",
        "police",
        "post_box",
        "post_office",
        "pub",
        "ranger_station",
        "recycling",
        "restaurant",
        "school",
        "shelter",
        "swimming_pool",
        "taxi",
        "telephone",
        "theatre",
        "toilets",
        "townhall",
        "university",
        "veterinary",
        "waste_basket",
        "waste_disposal",
        "water_point",
        "watering_place",
    }),
    boundary = set({ "marker" }),
    emergency = set({ "access_point" }),
    highway = set({ "bus_stop", "rest_area" }),
    historic = set({
        "archaeological_site",
        "boundary_stone",
        "bunker",
        "castle",
        "manor",
        "memorial",
        "mine",
        "mine_shaft",
        "monastery",
        "monument",
        "tree_shrine",
        "wayside_cross",
        "wayside_shrine",
    }),
    leisure = set({
        "beach_resort",
        "bird_hide",
        "dog_park",
        "firepit",
        "golf_course",
        "horse_riding",
        "miniature_golf",
        "outdoor_seating",
        "picnic_table",
        "playground",
        "resort",
        "sauna",
        "water_park",
    }),
    man_made = set({
        "adit",
        "apiary",
        "beehive",
        "chimney",
        "communications_tower",
        "cross",
        "forester's_lodge",
        "ice_house",
        "mine",
        "mineshaft",
        "pumping_station",
        "reservoir_covered",
        "silo",
        "wastewater_plant",
        "water_tower",
        "water_well",
        "water_works",
    }),
    military = set({ "bunker" }),
    natural = set({
        "arch",
        "birds_nest",
        "cave_entrance",
        "peak",
        "rock",
        "saddle",
        "shrub",
        "sinkhole",
        "stone",
        "tree",
    }),
    power = set({ "pole", "tower" }),
    railway = set({ "halt", "level_crossing", "station", "subway_entrance", "tram_stop" }),
    tourism = set({
        "alpine_hut",
        "apartment",
        "artwork",
        "attraction",
        "camp_site",
        "caravan_site",
        "castle",
        "chalet",
        "guest_house",
        "hostel",
        "hotel",
        "memorial",
        "monument",
        "motel",
        "museum",
        "picnic_site",
        "theme_park",
        "viewpoint",
        "wilderness_hut",
        "zoo",
    }),
    waterway = set({ "waterfall", "weir", "dam" }),
}

local feature_poly_values = {
    aeroway = set({ "aerodrome" }),
    amenity = set({
        "bank",
        "bar",
        "bbq",
        "bicycle_parking",
        "bicycle_rental",
        "biergarten",
        "bus_station",
        "cafe",
        "car_wash",
        "cinema",
        "clinic",
        "college",
        "community_centre",
        "dentist",
        "doctors",
        "fast_food",
        "feeding_place",
        "fire_station",
        "fountain",
        "fuel",
        "game_feeding",
        "hospital",
        "ice_cream",
        "kindergarten",
        "library",
        "parking",
        "pharmacy",
        "police",
        "post_office",
        "pub",
        "recycling",
        "restaurant",
        "school",
        "shelter",
        "swimming_pool",
        "taxi",
        "theatre",
        "toilets",
        "townhall",
        "university",
        "veterinary",
        "monastery",
        "ranger_station",
    }),
    highway = set({ "rest_area" }),
    historic = set({
        "archaeological_site",
        "bunker",
        "castle",
        "manor",
        "memorial",
        "mine",
        "mine_shaft",
        "monastery",
        "monument",
        "wayside_shrine",
    }),
    leisure = set({
        "beach_resort",
        "bird_hide",
        "dog_park",
        "firepit",
        "golf_course",
        "horse_riding",
        "miniature_golf",
        "outdoor_seating",
        "playground",
        "resort",
        "sauna",
        "track",
        "water_park",
    }),
    man_made = set({
        "adit",
        "apiary",
        "beehive",
        "chimney",
        "communications_tower",
        "cross",
        "forester's_lodge",
        "ice_house",
        "mine",
        "mineshaft",
        "pumping_station",
        "reservoir_covered",
        "silo",
        "wastewater_plant",
        "water_tower",
        "water_well",
        "water_works",
    }),
    military = set({ "bunker" }),
    natural = set({ "rock", "stone", "sinkhole" }),
    tourism = set({
        "alpine_hut",
        "apartment",
        "artwork",
        "attraction",
        "camp_site",
        "caravan_site",
        "castle",
        "chalet",
        "guest_house",
        "hostel",
        "hotel",
        "monument",
        "motel",
        "museum",
        "picnic_site",
        "theme_park",
        "wilderness_hut",
        "zoo",
    }),
    railway = set({ "station" }),
    waterway = set({ "weir", "dam" }),
}

---@param tags table<string, string>
---@param lookup table<string, string[]>
---@return string | nil, string | nil
local function matches_any(tags, lookup)
    for key, values in pairs(lookup) do
        local v = tags[key]
        if v and values[v] then
            return key, v
        end
    end

    return nil, nil
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_landuse(obj, geom)
    local tags = obj.tags
    local key, val = matches_any(tags, landuse_values)
    if not key then
        return
    end

    local area = geom:spherical_area()

    local row = {
        osm_id = obj.id,
        geometry = geom,
        name = tags.name,
        type = val,
        area = area,
        tags = take_tags(tags, { "wetland" }),
    }

    tables.landusages:insert(row)
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_building(obj, geom)
    local val = obj.tags.building
    if not val then
        return
    end

    tables.buildings:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_waterarea(obj, geom)
    local tags = obj.tags

    local key, val = matches_any(tags, {
        landuse = { basin = true, reservoir = true },
        amenity = { swimming_pool = true, fountain = true },
        leisure = { swimming_pool = true },
        natural = { water = true },
        waterway = { riverbank = true },
    })

    if not key then
        return
    end

    local area = geom:spherical_area()

    local row = {
        geometry = geom,
        name = tags.name,
        type = val,
        area = area,
        intermittent = tags.intermittent,
        seasonal = tags.seasonal,
        water = tags.water,
    }

    tables.waterareas:insert(row)
    insert_generalized_waterarea(row, geom, area)
end

local accepted_waterways = set({ "river", "canal", "stream", "drain", "ditch" })

---@param obj OsmObject
---@param geom OsmGeometry
local function process_waterway(obj, geom)
    local tags = obj.tags

    local val = tags.waterway

    if not accepted_waterways[val] then
        return
    end

    local row = {
        geometry = geom,
        name = tags.name,
        intermittent = tags.intermittent,
        seasonal = tags.seasonal,
        tunnel = tags.tunnel,
        type = val,
    }

    tables.waterways:insert(row)

    if val == "river" then
        insert_generalized_waterway(row, geom)
    end
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_barrier_way(obj, geom)
    local val = obj.tags.barrier
    if not val then
        return
    end

    tables.barrierways:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_barrier_point(obj, geom)
    local val = obj.tags.barrier
    if not val then
        return
    end

    tables.barrierpoints:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        access = obj.tags.access,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_feature_line(obj, geom)
    local key, val = matches_any(obj.tags, feature_line_values)
    if not key then
        return
    end

    tables.feature_lines:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        fixme = obj.tags.fixme,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_pipeline(obj, geom)
    if obj.tags.man_made ~= "pipeline" then
        return
    end

    tables.pipelines:insert({
        geometry = geom,
        name = obj.tags.name,
        location = obj.tags.location,
        substance = obj.tags.substance,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_protected_area(obj, geom)
    local tags = obj.tags
    local key, val = matches_any(tags, {
        boundary = { national_park = true, protected_area = true },
        leisure = { nature_reserve = true },
    })

    if not key then
        return
    end

    local area = geom:spherical_area()

    tables.protected_areas:insert({
        geometry = geom,
        name = tags.name,
        type = val,
        protect_class = tags.protect_class,
        area = area,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_ford(obj, geom)
    local val = obj.tags.ford
    if not val then
        return
    end

    tables.fords:insert({
        geometry = geom,
        type = val,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_feature_point(obj, geom)
    local key, val = matches_any(obj.tags, feature_point_values)
    if not key then
        return
    end

    tables.features:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        tags = take_tags(obj.tags, {
            "access",
            "ele",
            "icao",
            "protected",
            "shelter_type",
            "disused",
            "denotation",
            "fee",
            "ref",
        }),
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_feature_poly(obj, geom)
    local key, val = matches_any(obj.tags, feature_poly_values)
    if not key then
        return
    end

    tables.feature_polys:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        tags = take_tags(obj.tags, { "ele", "access", "shelter_type", "icao", "disused", "ref" }),
    })
end

-- as poi
---@param obj OsmObject
---@param geom OsmGeometry
local function process_tower(obj, geom)
    local man_made = obj.tags.man_made
    if man_made ~= "tower" and man_made ~= "mast" and man_made ~= "water_tower" then
        return
    end

    tables.towers:insert({
        geometry = to_surface_point(geom),
        name = obj.tags.name,
        class = man_made == "water_tower" and "tower" or man_made,
        type = obj.tags["tower:type"],
        ele = obj.tags.ele,
    })
end

local spring_pois = set({ "spring", "hot_spring", "geyser" });
local natural_pois = shallow_copy(spring_pois);
local man_made_pois = set({ "spring_box", "tower", "mast", "water_tower" });

---@param obj OsmObject
---@param geom OsmGeometry
local function process_poi(obj, geom)
    local tags = obj.tags

    local val = natural_pois[tags.natural] and tags.natural
        or man_made_pois[tags.man_made] and tags.man_made

    if not val then
        return
    end

    local extra = {}

    if val == "tower" then
        extra["tower:type"] = tags["tower:type"]
    elseif val == "spring" then
        extra = {
            ele = tags.ele,
            refitted = tags.refitted,
            seasonal = tags.seasonal,
            intermittent = tags.intermittent,
            drinking_water = tags.drinking_water,
            water_characteristic = tags.water_characteristic,
            tags["tower:type"]
        }
    end

    tables.pois:insert({
        geometry = geom,
        name = tags.name,
        type = val,
    })
end

-- as poi
---@param obj OsmObject
---@param geom OsmGeometry
local function process_spring(obj, geom)
    local tags = obj.tags
    local val = tags.natural == "spring" and "spring"
        or tags.natural == "hot_spring" and "hot_spring"
        or tags.natural == "geyser" and "geyser"
        or (tags.man_made == "spring_box" and "spring_box")

    if not val then
        return
    end

    tables.springs:insert({
        geometry = geom,
        name = tags.name,
        type = val,
        ele = tags.ele,
        refitted = tags.refitted,
        seasonal = tags.seasonal,
        intermittent = tags.intermittent,
        drinking_water = tags.drinking_water,
        water_characteristic = tags.water_characteristic,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_building_point(obj, geom)
    local val = obj.tags.building
    if not val then
        return
    end

    tables.building_points:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        tags = take_tags(obj.tags, { "access" }),
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_sport(obj, geom)
    local val = obj.tags.sport
    if not val then
        return
    end

    tables.sports:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        tags = take_tags(obj.tags, { "access" }),
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_power_generator(obj, geom)
    if obj.tags.power ~= "generator" then
        return
    end

    tables.power_generators:insert({
        geometry = geom,
        name = obj.tags.name,
        source = obj.tags["generator:source"],
        method = obj.tags["generator:method"],
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_ruins(obj, geom)
    if obj.tags.historic ~= "ruins" then
        return
    end

    tables.ruins:insert({
        geometry = to_surface_point(geom),
        name = obj.tags.name,
        type = obj.tags.ruins,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_place_of_worship(obj, geom)
    if obj.tags.amenity ~= "place_of_worship" then
        return
    end

    tables.place_of_worships:insert({
        geometry = geom,
        name = obj.tags.name,
        building = obj.tags.building,
        religion = obj.tags.religion,
    })
end

local infopoint_values = set({
    "guidepost",
    "board",
    "map",
    "office",
    "route_marker",
})

---@param obj OsmObject
---@param geom OsmGeometry
local function process_infopoint(obj, geom)
    if infopoint_values[obj.tags.information] == nil then
        return
    end

    tables.infopoints:insert({
        geometry = geom,
        name = obj.tags.name,
        ele = obj.tags.ele,
        foot = obj.tags.foot,
        bicycle = obj.tags.bicycle,
        ski = obj.tags.ski,
        horse = obj.tags.horse,
        type = obj.tags.information,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_aerialway(obj, geom)
    local val = obj.tags.aerialway
    if not val then
        return
    end

    tables.aerialways:insert({
        geometry = geom,
        type = val,
        name = obj.tags.name,
        ref = obj.tags.ref,
    })
end

local road_highway_values = set({
    "motorway",
    "motorway_link",
    "trunk",
    "trunk_link",
    "primary",
    "primary_link",
    "secondary",
    "secondary_link",
    "tertiary",
    "tertiary_link",
    "road",
    "path",
    "track",
    "service",
    "footway",
    "bridleway",
    "cycleway",
    "steps",
    "pedestrian",
    "living_street",
    "unclassified",
    "residential",
    "raceway",
    "platform",
    "construction",
    "piste",
    "escape",
    "corridor",
    "bus_guideway",
    "via_ferrata",
})

local trail_visibility_enum = {
    excellent = 1,
    good = 2,
    intermediate = 3,
    bad = 4,
    horrible = 5,
    ["no"] = 6,
}

local sac_scale_enum = {
    hiking = 1,
    mountain_hiking = 2,
    demanding_mountain_hiking = 3,
    alpine_hiking = 4,
    demanding_alpine_hiking = 5,
    difficult_alpine_hiking = 6,
}

---@param obj OsmObject
---@param geom OsmGeometry
local function process_road(obj, geom)
    local tags = obj.tags
    local class = nil
    local val = nil

    if tags.highway and road_highway_values[tags.highway] then
        class = "highway"
        val = tags.highway
    elseif tags.railway and tags.railway ~= "" then
        class = "railway"
        val = tags.railway
    elseif tags.route == "piste" then
        class = "route"
        val = "piste"
    elseif tags.man_made == "pier" or tags.man_made == "groyne" then
        class = "man_made"
        val = tags.man_made
    elseif tags.public_transport == "platform" then
        class = "public_transport"
        val = "platform"
    elseif tags.attraction == "water_slide" then
        class = "attraction"
        val = "water_slide"
    elseif tags.leisure == "track" then
        class = "leisure"
        val = "track"
    end

    if not class then
        return
    end

    if tags.area == "yes" then
        return
    end

    local row = {
        geometry = geom,
        type = val,
        name = tags.name,
        tunnel = tags.tunnel,
        embankment = tags.embankment,
        bridge = tags.bridge,
        oneway = tags.oneway,
        cutting = enum_from_tags(tags, "cutting",
            { "yes", "left", "right" }),
        ref = tags.ref,
        z_order = z_order_for_way(tags),
        access = tags.access,
        bicycle = tags.bicycle,
        foot = tags.foot,
        vehicle = tags.vehicle,
        service = tags.service,
        tracktype = tags.tracktype,
        class = class,
        trail_visibility = trail_visibility_enum[tags.trail_visibility],
        sac_scale = sac_scale_enum[tags.sac_scale],
        fixme = tags.fixme,
    }

    tables.roads:insert(row)

    if
        (
            class == "highway"
            and (
                val == "motorway"
                or val == "motorway_link"
                or val == "trunk"
                or val == "trunk_link"
                or val == "primary"
                or val == "primary_link"
                or val == "secondary"
                or val == "secondary_link"
                or val == "tertiary"
                or val == "tertiary_link"
            )
        ) or class == "railway"
    then
        row.geometry = geom:simplify(50)
        tables.roads_gen1:insert(row)

        row.geometry = geom:simplify(200)
        tables.roads_gen0:insert(row)
    end
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_housenumber(obj, geom)
    local tags = obj.tags
    local hn = tags["addr:housenumber"] or tags["addr:streetnumber"] or tags["addr:conscriptionnumber"]
    if not hn then
        return
    end

    tables.housenumbers:insert({
        geometry = to_surface_point(geom),
        housenumber = hn,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_shop(obj, geom)
    local val = obj.tags.shop
    if not val then
        return
    end

    tables.shops:insert({
        geometry = to_surface_point(geom),
        name = obj.tags.name,
        type = val,
    })
end

local place_order = {
    locality = 1,
    plot = 2,
    isolated_dwelling = 3,
    square = 4,
    farm = 5,
    city_block = 6,
    quarter = 7,
    neighbourhood = 8,
    allotments = 9,
    borough = 10,
    suburb = 11,
    hamlet = 12,
    village = 13,
    town = 14,
    city = 15,
    county = 16,
    region = 17,
    state = 18,
    country = 19,
}

---@param obj OsmObject
---@param geom OsmGeometry
local function process_place(obj, geom)
    local val = obj.tags.place
    if not val then
        return
    end

    tables.places:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
        z_order = place_order[val],
        population = tonumber(obj.tags.population),
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_aeroway(obj, geom)
    local val = obj.tags.aeroway
    if val ~= "runway" and val ~= "taxiway" then
        return
    end

    tables.aeroways:insert({
        geometry = geom,
        name = obj.tags.name,
        type = val,
    })
end

---@param obj OsmObject
---@param geom OsmGeometry
local function process_feature_point_fixme(obj, geom)
    if not obj.tags.fixme then
        return
    end

    tables.fixmes:insert({
        geometry = geom,
        type = obj.tags.fixme,
    })
end

function osm2pgsql.process_node(object)
    local geom = object:as_point()

    process_place(object, geom)
    process_barrier_point(object, geom)
    process_feature_point(object, geom)
    process_spring(object, geom)
    process_building_point(object, geom)
    process_tower(object, geom)
    process_sport(object, geom)
    process_power_generator(object, geom)
    process_ruins(object, geom)
    process_place_of_worship(object, geom)
    process_infopoint(object, geom)
    process_feature_point_fixme(object, geom)
    process_housenumber(object, geom)
    process_shop(object, geom)
    process_ford(object, geom)
end

---@type table<integer, integer[]>>
local w2r = {}

tables.route_way = osm2pgsql.define_table({
    name = "osm_route_way",
    ids = {
        id_column = "osm_id", type = 'way', create_index = 'primary_key'
    },
    columns = {
        { column = "geometry", type = "linestring", not_null = true, projection = projection }
    },
})

tables.route_to_way = osm2pgsql.define_table({
    name = "osm_route_to_way",
    ids = {
        id_column = "way_id", type = 'way', create_index = 'always'
    },
    columns = {
        { column = "rel_id", type = 'bigint', not_null = true },
    },
    indexes = {
        { column = { "rel_id", "way_id" }, method = "btree" },
        { column = "rel_id",               method = "btree" },
    }
})

function osm2pgsql.process_way(object)
    local tags = object.tags

    local can_be_polygon = object.is_closed and object.tags.area ~= "no";
    local can_be_linestring = not object.is_closed and object.tags.area ~= "yes";

    if can_be_polygon then
        local polygon = object:as_polygon()
        if not polygon then
            return
        end

        process_landuse(object, polygon)
        process_building(object, polygon)
        process_waterarea(object, polygon)
        process_protected_area(object, polygon)
        process_feature_poly(object, polygon)
        process_housenumber(object, polygon)
        process_shop(object, polygon)
        process_tower(object, polygon)
        process_sport(object, polygon)
        process_power_generator(object, polygon)
        process_ruins(object, polygon)
        process_place_of_worship(object, polygon)
        process_ford(object, polygon)
    elseif can_be_linestring then
        local line = object:as_linestring()
        if not line then
            return
        end

        process_waterway(object, line)
        process_barrier_way(object, line)
        process_feature_line(object, line)
        process_pipeline(object, line)
        process_aerialway(object, line)
        process_aeroway(object, line)
        process_road(object, line)
        process_ford(object, line)

        local rel_ids = w2r[object.id]

        if rel_ids then
            tables.route_way:insert({
                geometry = object:as_linestring()
            })

            for _, rel_id in ipairs(rel_ids) do
                tables.route_to_way:insert({
                    rel_id = rel_id,
                    way_id = object.id,
                })
            end
        end
    end
end

local outdoor_routes = set {
    "hiking",
    "bicycle",
    "ski",
    "horse",
    "piste",
    "foot",
    "mtb",
    "running"
}

function osm2pgsql.process_relation(object)
    local tags = object.tags

    if tags.type == "multipolygon" or tags.type == "boundary" then
        local multi = object:as_multipolygon()
        if multi then
            process_landuse(object, multi)
            process_building(object, multi)
            process_waterarea(object, multi)
            process_protected_area(object, multi)
            process_feature_poly(object, multi)
            process_housenumber(object, multi)
            process_shop(object, multi)
            process_tower(object, multi)
            process_sport(object, multi)
            process_power_generator(object, multi)
            process_ruins(object, multi)
            process_place_of_worship(object, multi)
            process_ford(object, multi)
        end
    end

    if tags.type == "route" and outdoor_routes[tags.route] then
        tables.routes:insert({
            name = tags.name,
            ref = tags.ref,
            colour = tags.colour,
            state = tags.state,
            ["osmc:symbol"] = tags["osmc:symbol"],
            network = tags.network,
            type = tags.route,
        })

        for _, member in ipairs(object.members) do
            if member.type == "w" then
                if not w2r[member.ref] then
                    w2r[member.ref] = {}
                end

                w2r[member.ref][#w2r[member.ref] + 1] = object.id;

                -- local row = {
                --     osm_id = object.id,
                --     member = member.ref,
                --     geometry = rel_geom,
                --     role = member.role,
                --     type = nil,
                -- }

                -- tables.route_members:insert(row)

                -- insert_generalized_route_member(row, rel_geom)
            end
        end
    end
end

function osm2pgsql.select_relation_members(relation)
    if relation.tags.type == 'route' and outdoor_routes[relation.tags.route] then
        return { ways = osm2pgsql.way_member_ids(relation) }
    end
end

function osm2pgsql.process_gen()
    osm2pgsql.run_gen(
        'raster-union',
        {
            name = "osm_landusages_gen0",
            cluster = "no",
            src_table = "osm_landusages",
            dest_table = "osm_landusages_gen0",
            geom_column = "geometry",
            zoom = 9
        }
    );

    osm2pgsql.run_gen(
        'raster-union',
        {
            name = "osm_landusages_gen1",
            cluster = "no",
            src_table = "osm_landusages",
            dest_table = "osm_landusages_gen1",
            geom_column = "geometry",
            zoom = 11
        }
    );
end
