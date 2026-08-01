import { Fragment, Slice, type Node as PMNode } from "@tiptap/pm/model";
import { NodeSelection, Selection, type EditorState, type Transaction } from "@tiptap/pm/state";
import type { Editor } from "@tiptap/core";

export interface DocumentImageSnapshot {
  imageId: string;
  fingerprint: string;
  ordinal: number;
  blockId: string | null;
  imagePos: number;
  standalone: boolean;
  alt: string | null;
  width: string | null;
  height: string | null;
  mediaRef: string | null;
  projectSourcePath: string | null;
  projectSourceHash: string | null;
  alignment: "left" | "center" | "right";
  capabilities: Array<"delete_image" | "resize_image" | "move_image" | "set_image_layout">;
  resizeBounds: { min: number; max: number };
  moveDirections: Array<"up" | "down">;
}

export interface InsertImageOperation {
  op: "insert_image";
  anchorBlockId: string;
  position: "before" | "after";
  assetId: string;
  sourcePath: string;
  sourceHash: string;
  mediaRef: string;
  displaySrc?: string;
  alt?: string;
  width?: number;
}

export interface DeleteImageOperation {
  op: "delete_image";
  imageId: string;
  fingerprint: string;
}

export interface ResizeImageOperation {
  op: "resize_image";
  imageId: string;
  fingerprint: string;
  width: number;
}

export interface MoveImageOperation {
  op: "move_image";
  imageId: string;
  fingerprint: string;
  direction: "up" | "down";
}

export interface SetImageLayoutOperation {
  op: "set_image_layout";
  imageId: string;
  fingerprint: string;
  alignment: "left" | "center" | "right";
}

export type ImageOperation =
  | InsertImageOperation
  | DeleteImageOperation
  | ResizeImageOperation
  | MoveImageOperation
  | SetImageLayoutOperation;

export interface ImageEditPayload {
  base: {
    sessionVersion: number;
    contentHash: string;
  };
  operations: ImageOperation[];
}

export interface ImageOperationApplyResult {
  before: DocumentImageSnapshot[];
  after: DocumentImageSnapshot[];
  affectedBlockIds: string[];
}

interface LocatedImage {
  image: PMNode;
  imagePos: number;
  parent: PMNode;
  parentPos: number;
  parentIndex: number;
  standalone: boolean;
}

interface ActiveImageSnapshot {
  blockId: string | null;
  imagePos: number;
  standalone: boolean;
  moveDirections: Array<"up" | "down">;
}

function stableHash(value: string): string {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= BigInt(value.charCodeAt(index));
    hash = BigInt.asUintN(64, hash * prime);
  }
  return hash.toString(16).padStart(16, "0");
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function isStandaloneImageParagraph(parent: PMNode): boolean {
  if (parent.type.name !== "paragraph") return false;
  let imageCount = 0;
  let hasMixedContent = false;
  parent.forEach((child) => {
    if (child.type.name === "image") {
      imageCount += 1;
      return;
    }
    if (child.isText && !(child.text ?? "").trim()) return;
    if (child.type.name === "hardBreak") return;
    hasMixedContent = true;
  });
  return imageCount === 1 && !hasMixedContent;
}

function normalizedAlignment(value: unknown): "left" | "center" | "right" {
  return value === "center" || value === "right" ? value : "left";
}

function topLevelIndexAt(doc: PMNode, parentPos: number): number {
  let found = -1;
  doc.forEach((_node, offset, index) => {
    if (offset === parentPos) found = index;
  });
  return found;
}

function editorContentWidth(editor: Editor): number {
  const dom = editor.view?.dom;
  if (!dom?.getBoundingClientRect) return 4096;
  const rectWidth = dom.getBoundingClientRect().width;
  const inlineZoom = Number.parseFloat((dom as HTMLElement).style?.zoom ?? "");
  const zoom = Number.isFinite(inlineZoom) && inlineZoom > 0 ? inlineZoom : 1;
  return Number.isFinite(rectWidth) && rectWidth > 0
    ? Math.max(40, Math.round(rectWidth / zoom))
    : 4096;
}

function imageSnapshotsFromDoc(doc: PMNode, maxWidth = 4096): DocumentImageSnapshot[] {
  const images: DocumentImageSnapshot[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== "image") return true;
    const $image = doc.resolve(pos);
    const parent = $image.parent;
    const standalone = (
      $image.depth === 1
      && isStandaloneImageParagraph(parent)
    );
    const parentPos = $image.depth > 0 ? $image.before($image.depth) : -1;
    const parentIndex = standalone ? topLevelIndexAt(doc, parentPos) : -1;
    let blockId: string | null = null;
    for (let depth = $image.depth; depth > 0; depth -= 1) {
      const candidate = $image.node(depth);
      const value = nullableString(candidate.attrs.blockId);
      if (value) {
        blockId = value;
        break;
      }
    }
    const ordinal = images.length;
    const alt = nullableString(node.attrs.alt);
    const width = nullableString(node.attrs.width);
    const height = nullableString(node.attrs.height);
    const mediaRef = nullableString(node.attrs.binderMediaSrc) ?? nullableString(node.attrs.src);
    const projectSourcePath = nullableString(node.attrs.binderProjectSrc);
    const projectSourceHash = nullableString(node.attrs.binderProjectHash);
    const alignment = normalizedAlignment(parent.attrs.textAlign);
    const moveDirections: Array<"up" | "down"> = [];
    if (standalone && parentIndex > 0) moveDirections.push("up");
    if (standalone && parentIndex >= 0 && parentIndex + 1 < doc.childCount) moveDirections.push("down");
    const capabilities: DocumentImageSnapshot["capabilities"] = ["delete_image", "resize_image"];
    if (standalone) capabilities.push("set_image_layout");
    if (moveDirections.length) capabilities.push("move_image");
    const imageId = nullableString(node.attrs.binderImageId) ?? `image_unassigned_${ordinal}`;
    const identity = JSON.stringify({
      imageId,
      ordinal,
      blockId,
      standalone,
      alignment,
      alt,
      width,
      height,
      mediaRef,
      projectSourcePath,
      projectSourceHash,
    });
    const fingerprint = stableHash(identity);
    images.push({
      imageId,
      fingerprint,
      ordinal,
      blockId,
      imagePos: pos,
      standalone,
      alt,
      width,
      height,
      mediaRef,
      projectSourcePath,
      projectSourceHash,
      alignment,
      capabilities,
      resizeBounds: { min: 40, max: Math.max(40, maxWidth) },
      moveDirections,
    });
    return false;
  });
  return images;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-QUERY-ED-ED-EDIT-024, BR-ED-DATA-003-QUERY-ED-ED-EDIT-024
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003
 * term_ref: TERM-ED-008
 * boundary: in=CurrentProseMirrorLogicalState | out=ImageSnapshotList(stableBlockAndCurrentInstanceFingerprint)
 */
export function projectDocumentImages(editor: Editor | null): DocumentImageSnapshot[] {
  return editor ? imageSnapshotsFromDoc(editor.state.doc, editorContentWidth(editor)) : [];
}

function nonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function safeProjectPath(value: string): boolean {
  return (
    !value.startsWith("/")
    && !value.startsWith("\\")
    && !value.includes("\\")
    && !value.includes("://")
    && !/^[A-Za-z]:/.test(value)
    && value.split("/").every((component) => component !== "" && component !== "." && component !== "..")
  );
}

/**
 * @GOV
 * codes:    BR-SYS-DATA-006-RB-ED-ED-EDIT-027, BR-ED-DATA-003-RB-ED-ED-EDIT-027
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-SYS-DATA-006, BR-ED-DATA-003
 * term_ref: TERM-SYS-002, TERM-ED-008
 * boundary: in=UntrustedImageOperationDiffPayload | out=DefensivelyValidatedImageEditPayloadOrRejection | consumer=BDocxCapabilityManifestImageOperationContract
 */
export function parseImageEditPayload(value: unknown): ImageEditPayload | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const payload = value as Record<string, unknown>;
  const base = payload.base as Record<string, unknown> | undefined;
  if (
    !base
    || !Number.isSafeInteger(base.sessionVersion)
    || (base.sessionVersion as number) < 0
    || !nonEmptyString(base.contentHash)
    || !Array.isArray(payload.operations)
    || payload.operations.length < 1
    || payload.operations.length > 16
  ) return null;
  const operations: ImageOperation[] = [];
  for (const raw of payload.operations) {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
    const operation = raw as Record<string, unknown>;
    if (operation.op === "insert_image") {
      if (
        !nonEmptyString(operation.anchorBlockId)
        || (operation.position !== "before" && operation.position !== "after")
        || !nonEmptyString(operation.assetId)
        || !nonEmptyString(operation.sourcePath)
        || !safeProjectPath(operation.sourcePath)
        || typeof operation.sourceHash !== "string"
        || !/^[0-9a-f]{64}$/.test(operation.sourceHash)
        || !nonEmptyString(operation.mediaRef)
        || !new RegExp(`^media/binder-project-${operation.sourceHash}\\.(?:png|jpg)$`).test(operation.mediaRef)
        || (operation.alt !== undefined && (
          typeof operation.alt !== "string"
          || operation.alt.length > 512
          || /[\u0000-\u001f\u007f]/.test(operation.alt)
        ))
        || (operation.width !== undefined && (
          !Number.isSafeInteger(operation.width)
          || (operation.width as number) < 1
          || (operation.width as number) > 4096
        ))
        || (operation.displaySrc !== undefined && !nonEmptyString(operation.displaySrc))
      ) return null;
      operations.push({
        op: "insert_image",
        anchorBlockId: operation.anchorBlockId,
        position: operation.position,
        assetId: operation.assetId,
        sourcePath: operation.sourcePath,
        sourceHash: operation.sourceHash,
        mediaRef: operation.mediaRef,
        ...(operation.displaySrc === undefined ? {} : { displaySrc: operation.displaySrc }),
        ...(operation.alt === undefined ? {} : { alt: operation.alt as string }),
        ...(operation.width === undefined ? {} : { width: operation.width as number }),
      });
    } else if (operation.op === "delete_image") {
      if (!nonEmptyString(operation.imageId) || !nonEmptyString(operation.fingerprint)) return null;
      operations.push({
        op: "delete_image",
        imageId: operation.imageId,
        fingerprint: operation.fingerprint,
      });
    } else if (operation.op === "resize_image") {
      if (
        !nonEmptyString(operation.imageId)
        || !nonEmptyString(operation.fingerprint)
        || !Number.isSafeInteger(operation.width)
        || (operation.width as number) < 1
        || (operation.width as number) > 4096
      ) return null;
      operations.push({
        op: "resize_image",
        imageId: operation.imageId,
        fingerprint: operation.fingerprint,
        width: operation.width as number,
      });
    } else if (operation.op === "move_image") {
      if (
        !nonEmptyString(operation.imageId)
        || !nonEmptyString(operation.fingerprint)
        || (operation.direction !== "up" && operation.direction !== "down")
      ) return null;
      operations.push({
        op: "move_image",
        imageId: operation.imageId,
        fingerprint: operation.fingerprint,
        direction: operation.direction,
      });
    } else if (operation.op === "set_image_layout") {
      if (
        !nonEmptyString(operation.imageId)
        || !nonEmptyString(operation.fingerprint)
        || (
          operation.alignment !== "left"
          && operation.alignment !== "center"
          && operation.alignment !== "right"
        )
      ) return null;
      operations.push({
        op: "set_image_layout",
        imageId: operation.imageId,
        fingerprint: operation.fingerprint,
        alignment: operation.alignment,
      });
    } else {
      return null;
    }
  }
  return {
    base: {
      sessionVersion: base.sessionVersion as number,
      contentHash: base.contentHash,
    },
    operations,
  };
}

function parseSnapshotString(
  value: unknown,
  maxLength: number,
  nullable: boolean,
): string | null | undefined {
  if (value === null && nullable) return null;
  if (
    typeof value !== "string"
    || value.length < 1
    || value.length > maxLength
    || /[\u0000-\u001f\u007f]/.test(value)
  ) return undefined;
  return value;
}

function parseDocumentImageSnapshot(value: unknown): DocumentImageSnapshot | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const snapshot = value as Record<string, unknown>;
  const imageId = parseSnapshotString(snapshot.imageId, 256, false);
  const fingerprint = parseSnapshotString(snapshot.fingerprint, 256, false);
  const blockId = parseSnapshotString(snapshot.blockId, 512, true);
  const alt = parseSnapshotString(snapshot.alt, 512, true);
  const width = parseSnapshotString(snapshot.width, 64, true);
  const height = parseSnapshotString(snapshot.height, 64, true);
  const mediaRef = parseSnapshotString(snapshot.mediaRef, 4096, true);
  const projectSourcePath = parseSnapshotString(snapshot.projectSourcePath, 4096, true);
  const projectSourceHash = parseSnapshotString(snapshot.projectSourceHash, 64, true);
  const alignment = snapshot.alignment === undefined
    ? "left"
    : normalizedAlignment(snapshot.alignment);
  if (
    snapshot.alignment !== undefined
    && snapshot.alignment !== "left"
    && snapshot.alignment !== "center"
    && snapshot.alignment !== "right"
  ) return null;
  const moveDirections = snapshot.moveDirections === undefined
    ? []
    : snapshot.moveDirections;
  if (
    !Array.isArray(moveDirections)
    || moveDirections.some((direction) => direction !== "up" && direction !== "down")
    || new Set(moveDirections).size !== moveDirections.length
  ) return null;
  const resizeBounds = snapshot.resizeBounds === undefined
    ? { min: 40, max: 4096 }
    : snapshot.resizeBounds;
  if (
    !resizeBounds
    || typeof resizeBounds !== "object"
    || Array.isArray(resizeBounds)
    || !Number.isSafeInteger((resizeBounds as { min?: unknown }).min)
    || !Number.isSafeInteger((resizeBounds as { max?: unknown }).max)
    || Number((resizeBounds as { min: number }).min) < 1
    || Number((resizeBounds as { max: number }).max) < Number((resizeBounds as { min: number }).min)
  ) return null;
  const defaultCapabilities: DocumentImageSnapshot["capabilities"] = ["delete_image", "resize_image"];
  if (snapshot.standalone === true) defaultCapabilities.push("set_image_layout");
  if (moveDirections.length) defaultCapabilities.push("move_image");
  const capabilities = snapshot.capabilities === undefined
    ? defaultCapabilities
    : snapshot.capabilities;
  const allowedCapabilities = new Set<DocumentImageSnapshot["capabilities"][number]>([
    "delete_image",
    "resize_image",
    "move_image",
    "set_image_layout",
  ]);
  if (
    !Array.isArray(capabilities)
    || capabilities.some((capability) => !allowedCapabilities.has(capability))
    || new Set(capabilities).size !== capabilities.length
  ) return null;
  if (
    imageId === undefined
    || imageId === null
    || fingerprint === undefined
    || fingerprint === null
    || blockId === undefined
    || alt === undefined
    || width === undefined
    || height === undefined
    || mediaRef === undefined
    || projectSourcePath === undefined
    || projectSourceHash === undefined
    || !Number.isSafeInteger(snapshot.ordinal)
    || (snapshot.ordinal as number) < 0
    || !Number.isSafeInteger(snapshot.imagePos)
    || (snapshot.imagePos as number) < 0
    || typeof snapshot.standalone !== "boolean"
    || (mediaRef !== null && (!mediaRef.startsWith("media/") || !safeProjectPath(mediaRef)))
    || (projectSourcePath !== null && !safeProjectPath(projectSourcePath))
    || (projectSourceHash !== null && !/^[0-9a-f]{64}$/.test(projectSourceHash))
  ) return null;
  return {
    imageId,
    fingerprint,
    ordinal: snapshot.ordinal as number,
    blockId,
    imagePos: snapshot.imagePos as number,
    standalone: snapshot.standalone,
    alt,
    width,
    height,
    mediaRef,
    projectSourcePath,
    projectSourceHash,
    alignment,
    capabilities: capabilities as DocumentImageSnapshot["capabilities"],
    resizeBounds: resizeBounds as DocumentImageSnapshot["resizeBounds"],
    moveDirections: moveDirections as DocumentImageSnapshot["moveDirections"],
  };
}

/**
 * @GOV
 * codes:    BR-DE-DATA-002-RB-ED-DE-DIFF-028
 * type:     RB
 * chain:    DE-DIFF
 * rules:    BR-DE-DATA-002
 * term_ref: TERM-DE-004, TERM-ED-008
 * boundary: in=UntrustedPersistedImageReviewSnapshotProjection | out=ValidatedBeforeAfterImageSnapshotsAndAffectedBlockIdsOrRejection
 */
export function parseImageDiffSnapshotProjection(
  beforeValue: unknown,
  afterValue: unknown,
  affectedBlockIdsValue: unknown,
): ImageOperationApplyResult | null {
  if (
    !Array.isArray(beforeValue)
    || !Array.isArray(afterValue)
    || !Array.isArray(affectedBlockIdsValue)
    || beforeValue.length > 4096
    || afterValue.length > 4096
    || affectedBlockIdsValue.length > 64
  ) return null;
  const before = beforeValue.map(parseDocumentImageSnapshot);
  const after = afterValue.map(parseDocumentImageSnapshot);
  if (before.some((snapshot) => snapshot === null) || after.some((snapshot) => snapshot === null)) {
    return null;
  }
  const beforeIds = new Set(before.map((snapshot) => snapshot?.imageId));
  const afterIds = new Set(after.map((snapshot) => snapshot?.imageId));
  if (beforeIds.size !== before.length || afterIds.size !== after.length) return null;
  const affectedBlockIds: string[] = [];
  const affectedIds = new Set<string>();
  for (const value of affectedBlockIdsValue) {
    const blockId = parseSnapshotString(value, 512, false);
    if (blockId === undefined || blockId === null || affectedIds.has(blockId)) return null;
    affectedIds.add(blockId);
    affectedBlockIds.push(blockId);
  }
  return {
    before: before as DocumentImageSnapshot[],
    after: after as DocumentImageSnapshot[],
    affectedBlockIds,
  };
}

function locateSelectedImage(state: EditorState): LocatedImage | null {
  const selection = state.selection;
  if (!(selection instanceof NodeSelection) || selection.node.type.name !== "image") return null;
  const $image = state.doc.resolve(selection.from);
  const parent = $image.parent;
  const standalone = (
    $image.depth === 1
    && isStandaloneImageParagraph(parent)
  );
  const parentPos = $image.depth > 0 ? $image.before($image.depth) : -1;
  let parentIndex = -1;
  state.doc.forEach((_node, offset, index) => {
    if (offset === parentPos) parentIndex = index;
  });
  return {
    image: selection.node,
    imagePos: selection.from,
    parent,
    parentPos,
    parentIndex,
    standalone: standalone && parentIndex >= 0,
  };
}

function directionsFor(state: EditorState, located: LocatedImage): Array<"up" | "down"> {
  if (!located.standalone) return [];
  const directions: Array<"up" | "down"> = [];
  if (located.parentIndex > 0) directions.push("up");
  if (located.parentIndex + 1 < state.doc.childCount) directions.push("down");
  return directions;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-QUERY-ED-ED-EDIT-021, BR-ED-DATA-003-QUERY-ED-ED-EDIT-021
 * type:     QUERY
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003
 * boundary: in=EditorStateImageNodeSelection | out=ActiveImageSnapshot(standaloneBlockAndDocumentFlowDirections)
 */
export function activeImageSnapshot(editor: Editor | null): ActiveImageSnapshot | null {
  if (!editor) return null;
  const located = locateSelectedImage(editor.state);
  if (!located) return null;
  const blockId = typeof located.parent.attrs.blockId === "string" && located.parent.attrs.blockId
    ? located.parent.attrs.blockId
    : null;
  return {
    blockId,
    imagePos: located.imagePos,
    standalone: located.standalone,
    moveDirections: directionsFor(editor.state, located),
  };
}

function topLevelChildren(doc: PMNode): PMNode[] {
  const children: PMNode[] = [];
  doc.forEach((node) => children.push(node));
  return children;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-EFFECT-ED-ED-EDIT-022, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-022, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-022
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001
 * boundary: in=SelectedStandaloneImageBlockAndMoveDirection | out=LogicalStateTransactionWithMovedImageBlockAndNodeSelection | effect=memory | does_not=write_DiskState_or_media_resource
 */
export function moveActiveImage(editor: Editor | null, direction: "up" | "down"): boolean {
  if (!editor) return false;
  const located = locateSelectedImage(editor.state);
  if (!located || !directionsFor(editor.state, located).includes(direction)) return false;
  const children = topLevelChildren(editor.state.doc);
  const targetIndex = direction === "up" ? located.parentIndex - 1 : located.parentIndex + 1;
  [children[located.parentIndex], children[targetIndex]] = [
    children[targetIndex],
    children[located.parentIndex],
  ];

  let movedParentPos = 0;
  for (let index = 0; index < targetIndex; index += 1) movedParentPos += children[index].nodeSize;
  const transaction = editor.state.tr.replace(
    0,
    editor.state.doc.content.size,
    new Slice(Fragment.fromArray(children), 0, 0),
  );
  const movedParent = transaction.doc.nodeAt(movedParentPos);
  if (!movedParent || !isStandaloneImageParagraph(movedParent)) return false;
  let movedImageOffset = -1;
  movedParent.forEach((child, offset) => {
    if (movedImageOffset < 0 && child.type.name === "image") movedImageOffset = offset;
  });
  if (movedImageOffset < 0) return false;
  const movedImagePos = movedParentPos + 1 + movedImageOffset;
  if (transaction.doc.nodeAt(movedImagePos)?.type.name !== "image") return false;
  transaction.setSelection(NodeSelection.create(transaction.doc, movedImagePos));
  editor.view.dispatch(transaction);
  editor.view.focus();
  return true;
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-EFFECT-ED-ED-EDIT-023, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-023, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-023
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001
 * boundary: in=SelectedImageNodeDeleteIntent | out=LogicalStateTransactionWithoutSelectedImageAndNearbySelection | effect=memory | does_not=delete_media_resource_or_write_DiskState
 */
export function deleteActiveImage(editor: Editor | null): boolean {
  if (!editor) return false;
  const located = locateSelectedImage(editor.state);
  if (!located) return false;
  const transaction = (
    located.standalone && editor.state.doc.childCount > 1
      ? editor.state.tr.delete(located.parentPos, located.parentPos + located.parent.nodeSize)
      : editor.state.tr.delete(located.imagePos, located.imagePos + located.image.nodeSize)
  );
  const selectionPos = Math.min(located.parentPos + 1, transaction.doc.content.size);
  transaction.setSelection(Selection.near(transaction.doc.resolve(Math.max(0, selectionPos)), 1));
  editor.view.dispatch(transaction);
  editor.view.focus();
  return true;
}

interface TopLevelBlockLocation {
  node: PMNode;
  pos: number;
  index: number;
}

function topLevelBlockById(doc: PMNode, blockId: string): TopLevelBlockLocation | null {
  let found: TopLevelBlockLocation | null = null;
  doc.forEach((node, pos, index) => {
    if (!found && node.attrs.blockId === blockId) found = { node, pos, index };
  });
  return found;
}

let imageBlockSequence = 0;
let imageNodeSequence = 0;

function newImageBlockId(): string {
  const uuid = globalThis.crypto?.randomUUID?.();
  if (uuid) return `block_${uuid}`;
  imageBlockSequence += 1;
  return `block_image_${Date.now().toString(36)}_${imageBlockSequence.toString(36)}`;
}

function newImageId(): string {
  const uuid = globalThis.crypto?.randomUUID?.();
  if (uuid) return `image_${uuid}`;
  imageNodeSequence += 1;
  return `image_node_${Date.now().toString(36)}_${imageNodeSequence.toString(36)}`;
}

function locateImageById(doc: PMNode, imageId: string): LocatedImage | null {
  let found: LocatedImage | null = null;
  doc.descendants((node, pos) => {
    if (found || node.type.name !== "image" || node.attrs.binderImageId !== imageId) return;
    const $image = doc.resolve(pos);
    const parent = $image.parent;
    const parentPos = $image.depth > 0 ? $image.before($image.depth) : -1;
    const parentIndex = topLevelIndexAt(doc, parentPos);
    found = {
      image: node,
      imagePos: pos,
      parent,
      parentPos,
      parentIndex,
      standalone: $image.depth === 1 && parentIndex >= 0 && isStandaloneImageParagraph(parent),
    };
  });
  return found;
}

function deleteImageById(
  transaction: Transaction,
  imageId: string,
): { blockId: string | null } | null {
  const located = locateImageById(transaction.doc, imageId);
  if (!located) return null;
  const { imagePos, image, parent, parentPos, standalone } = located;
  const blockId = nullableString(parent.attrs.blockId);
  if (standalone && transaction.doc.childCount > 1) {
    transaction.delete(parentPos, parentPos + parent.nodeSize);
  } else {
    transaction.delete(imagePos, imagePos + image.nodeSize);
  }
  return { blockId };
}

/**
 * @GOV
 * codes:    BR-ED-DATA-001-EFFECT-ED-ED-EDIT-025, BR-ED-DATA-003-EFFECT-ED-ED-EDIT-025, BR-ED-STATE-001-EFFECT-ED-ED-EDIT-025
 * type:     EFFECT
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-001, BR-ED-DATA-003, BR-ED-STATE-001
 * term_ref: TERM-ED-008
 * boundary: in=ValidatedImageOperationBatchAndCurrentLogicalState | out=SingleAtomicLogicalStateTransactionWithBeforeAfterImageSnapshots | effect=memory | does_not=write_DiskState_or_delete_project_source
 */
export function applyImageOperations(
  editor: Editor | null,
  payload: ImageEditPayload,
): ImageOperationApplyResult | null {
  if (!editor || payload.operations.length < 1) return null;
  const maxWidth = editorContentWidth(editor);
  const before = imageSnapshotsFromDoc(editor.state.doc, maxWidth);
  const plannedTargets = new Map<string, {
    target: DocumentImageSnapshot;
    operations: Set<Exclude<ImageOperation["op"], "insert_image">>;
  }>();
  const affectedBlockIds = new Set<string>();

  for (const operation of payload.operations) {
    if (operation.op === "insert_image") {
      if (!topLevelBlockById(editor.state.doc, operation.anchorBlockId)) return null;
      affectedBlockIds.add(operation.anchorBlockId);
      continue;
    }
    const target = before.find((image) => (
      image.imageId === operation.imageId && image.fingerprint === operation.fingerprint
    ));
    if (!target || !target.capabilities.includes(operation.op)) return null;
    const planned = plannedTargets.get(operation.imageId);
    const operations = planned?.operations
      ?? new Set<Exclude<ImageOperation["op"], "insert_image">>();
    if (
      operations.has(operation.op)
      || operations.has("delete_image")
      || (operation.op === "delete_image" && operations.size > 0)
    ) return null;
    if (
      operation.op === "resize_image"
      && (operation.width < target.resizeBounds.min || operation.width > target.resizeBounds.max)
    ) return null;
    if (
      operation.op === "move_image"
      && !target.moveDirections.includes(operation.direction)
    ) return null;
    if (operation.op === "set_image_layout" && !target.standalone) return null;
    operations.add(operation.op);
    plannedTargets.set(operation.imageId, { target, operations });
    if (target.blockId) affectedBlockIds.add(target.blockId);
  }

  const transaction = editor.state.tr;
  const afterInsertSizes = new Map<string, number>();
  const operationPriority: Record<ImageOperation["op"], number> = {
    insert_image: 0,
    resize_image: 1,
    set_image_layout: 2,
    move_image: 3,
    delete_image: 4,
  };
  const orderedOperations = payload.operations
    .map((operation, index) => ({ operation, index }))
    .sort((left, right) => (
      operationPriority[left.operation.op] - operationPriority[right.operation.op]
      || left.index - right.index
    ))
    .map(({ operation }) => operation);
  for (const operation of orderedOperations) {
    if (operation.op === "insert_image") {
      const anchor = topLevelBlockById(transaction.doc, operation.anchorBlockId);
      if (!anchor) return null;
      const imageType = transaction.doc.type.schema.nodes.image;
      const paragraphType = transaction.doc.type.schema.nodes.paragraph;
      if (!imageType || !paragraphType) return null;
      const blockId = newImageBlockId();
      const image = imageType.create({
        src: operation.displaySrc ?? operation.mediaRef,
        alt: operation.alt ?? null,
        width: operation.width === undefined ? null : String(operation.width),
        height: null,
        binderMediaSrc: operation.mediaRef,
        binderProjectSrc: operation.sourcePath,
        binderProjectHash: operation.sourceHash,
        binderImageId: newImageId(),
        display: "inline",
      });
      const paragraph = paragraphType.create({ blockId, textAlign: "left" }, image);
      const priorAfterSize = afterInsertSizes.get(operation.anchorBlockId) ?? 0;
      const insertPos = operation.position === "before"
        ? anchor.pos
        : anchor.pos + anchor.node.nodeSize + priorAfterSize;
      transaction.insert(insertPos, paragraph);
      if (operation.position === "after") {
        afterInsertSizes.set(operation.anchorBlockId, priorAfterSize + paragraph.nodeSize);
      }
      affectedBlockIds.add(blockId);
    } else if (operation.op === "delete_image") {
      if (!deleteImageById(transaction, operation.imageId)) return null;
    } else if (operation.op === "resize_image") {
      const located = locateImageById(transaction.doc, operation.imageId);
      if (!located) return null;
      transaction.setNodeMarkup(located.imagePos, undefined, {
        ...located.image.attrs,
        width: String(operation.width),
        height: null,
      });
    } else if (operation.op === "set_image_layout") {
      const located = locateImageById(transaction.doc, operation.imageId);
      if (!located?.standalone) return null;
      transaction.setNodeMarkup(located.parentPos, undefined, {
        ...located.parent.attrs,
        textAlign: operation.alignment,
      });
    } else {
      const located = locateImageById(transaction.doc, operation.imageId);
      if (!located?.standalone) return null;
      const children = topLevelChildren(transaction.doc);
      const targetIndex = operation.direction === "up"
        ? located.parentIndex - 1
        : located.parentIndex + 1;
      if (targetIndex < 0 || targetIndex >= children.length) return null;
      const adjacentBlockId = nullableString(children[targetIndex].attrs.blockId);
      if (adjacentBlockId) affectedBlockIds.add(adjacentBlockId);
      [children[located.parentIndex], children[targetIndex]] = [
        children[targetIndex],
        children[located.parentIndex],
      ];
      transaction.replace(
        0,
        transaction.doc.content.size,
        new Slice(Fragment.fromArray(children), 0, 0),
      );
    }
  }
  if (!transaction.docChanged) return null;
  editor.view.dispatch(transaction);
  const after = imageSnapshotsFromDoc(editor.state.doc, maxWidth);
  return {
    before,
    after,
    affectedBlockIds: [...affectedBlockIds],
  };
}
